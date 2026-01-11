#include "http_app.h"
#include "esp_http_client.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_log.h"
#include "cJSON.h"
#include "wifi_set.h"
#include "esp_timer.h"
#include "default.h"
#include "memory.h"

static const char *TAG = "HTTP_APP";

// --- CONFIGURATION ---
#define SERVER_PORT 8765

char SERVER_IP[32] = {0};
char DEVICE_TOKEN[64] = {0};

#define CRASH_TIMEOUT_MS 15000

#define MAX_RETRIES 2

static int64_t last_heartbeat = 0;

// Helper to get current time in milliseconds
static int64_t get_time_ms()
{
    return esp_timer_get_time() / 1000;
}

// ---------------------------------------------------------
// 1. BUTTON LOGIC: Send "NEXT" Command (HTTP POST)
// ---------------------------------------------------------
void http_send_next_command(void)
{
    char url[128];
    // URL is now clean (no parameters)
    snprintf(url, sizeof(url), "http://%s:%d/next", SERVER_IP, SERVER_PORT);

    esp_http_client_config_t config = {
        .url = url,
        .method = HTTP_METHOD_POST,
        .timeout_ms = 5000,
    };

    esp_http_client_handle_t client = esp_http_client_init(&config);

    // Prepare the Authorization Header
    char auth_header[128];
    snprintf(auth_header, sizeof(auth_header), "Bearer %s", DEVICE_TOKEN);
    
    // Set the header
    esp_http_client_set_header(client, "Authorization", auth_header);

    // Perform the request
    esp_err_t err = esp_http_client_perform(client);

    if (err == ESP_OK)
    {
        int status_code = esp_http_client_get_status_code(client);
        ESP_LOGI(TAG, "Command sent! Status: %d", status_code);
        
        if (status_code == 401) {
             ESP_LOGE(TAG, "Server rejected token (401 Unauthorized)");
        }
    }
    else
    {
        ESP_LOGE(TAG, "Failed to send command: %s", esp_err_to_name(err));
    }

    esp_http_client_cleanup(client);
}

// ---------------------------------------------------------
// 2. SCREEN LOGIC: SSE Listener Task
// ---------------------------------------------------------

// Process a single line from SSE
void process_sse_line(char *line)
{
    // Basic Keep-Alive check
    if (strstr(line, "PING") != NULL || strstr(line, "connected") != NULL)
    {
        ESP_LOGI(TAG, "Heartbeat/Data Received");
        last_heartbeat = get_time_ms();
        return;
    }
}

void sse_task(void *pvParameters)
{
    char token[64] = {0}; // Increased size for UUID
    char broker_ip[32] = {0};

    esp_err_t err_token = nvs_get_info(SSE_INFO_MEMORY_REFERENCE, "token", token, sizeof(token));
    esp_err_t err_broker = nvs_get_info(SSE_INFO_MEMORY_REFERENCE, "broker_ip", broker_ip, sizeof(broker_ip));

    if (err_token != ESP_OK || err_broker != ESP_OK || strlen(broker_ip) == 0) {
        ESP_LOGE(TAG, "CRITICAL: Failed to load IP/Token from NVS. Aborting SSE Task.");
        wifi_ap_mode(); 
        vTaskDelete(NULL);
        return; 
    }

    strcpy(DEVICE_TOKEN, token); // Save globally for the Button function
    strcpy(SERVER_IP, broker_ip);

    // Update URL to include ?token=...
    char url[256];
    snprintf(url, sizeof(url), "http://%s:%d/events?token=%s", SERVER_IP, SERVER_PORT, DEVICE_TOKEN);

    esp_http_client_config_t config = {
        .url = url,
        .timeout_ms = 10000,
        .keep_alive_enable = true,
    };

    esp_http_client_handle_t client = esp_http_client_init(&config);

    int retry_count = 0;

    // Initial Heartbeat Reset
    last_heartbeat = get_time_ms();

    while (1)
    {
        ESP_LOGI(TAG, "Connecting to SSE Server...");

        if (esp_http_client_open(client, 0) != ESP_OK)
        {
            ESP_LOGE(TAG, "Failed to open connection");

            retry_count++;
            if (retry_count >= MAX_RETRIES)
            {
                ESP_LOGE(TAG, ">>> MAX RETRIES REACHED. ENTERING AP MODE <<<");
                esp_http_client_cleanup(client);

                wifi_ap_mode();
                vTaskDelete(NULL);
                return;
            }
            goto retry;
        }

        if (esp_http_client_fetch_headers(client) < 0)
        {
            ESP_LOGE(TAG, "Failed to fetch headers");
            goto retry;
        }
        
        // Check if server accepted the token
        int status_code = esp_http_client_get_status_code(client);
        if (status_code == 401) {
             ESP_LOGE(TAG, "SSE Auth Failed (401). Check Token.");
             // Stop retrying if token is wrong, it won't fix itself
             wifi_ap_mode();
             vTaskDelete(NULL);
             return;
        }

        char buffer[1];
        char line_buffer[256];
        int line_pos = 0;

        while (1)
        {
            int read_len = esp_http_client_read(client, buffer, 1);

            if (read_len <= 0)
            {
                if (esp_http_client_is_complete_data_received(client))
                {
                    ESP_LOGW(TAG, "Stream ended by server.");
                }
                else
                {
                    ESP_LOGE(TAG, "Stream read error/timeout.");
                }
                break;
            }

            // Watchdog: If no data received for 15s
            if ((get_time_ms() - last_heartbeat) > CRASH_TIMEOUT_MS)
            {
                ESP_LOGE(TAG, "CRASH DETECTED: No Heartbeat for 15s!");
                esp_http_client_close(client);
                esp_http_client_cleanup(client);

                wifi_ap_mode();
                vTaskDelete(NULL);
                return;
            }

            // Line Parsing Logic
            if (buffer[0] == '\n')
            {
                line_buffer[line_pos] = '\0';
                if (line_pos > 0) // Process non-empty lines
                {
                    process_sse_line(line_buffer);
                }
                line_pos = 0;
            }
            else if (buffer[0] != '\r')
            {
                if (line_pos < sizeof(line_buffer) - 1)
                {
                    line_buffer[line_pos++] = buffer[0];
                }
            }
        }

    retry:
        esp_http_client_close(client);
        ESP_LOGW(TAG, "Lost connection... Retrying in 1 second");
        vTaskDelay(pdMS_TO_TICKS(1000));
    }

    esp_http_client_cleanup(client);
    vTaskDelete(NULL);
}

void http_app_start_listener(void)
{
    xTaskCreate(sse_task, "sse_task", 4096, NULL, 5, NULL);
}