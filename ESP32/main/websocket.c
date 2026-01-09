#include "websocket.h"
#include "esp_http_client.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_log.h"
#include "cJSON.h"
#include "wifi_set.h"
#include "esp_timer.h"

static const char *TAG = "HTTP_APP";

// --- CONFIGURATION ---
#define SERVER_IP "192.168.0.199"
#define SERVER_PORT 8765
#define DEVICE_TOKEN "0hW1DI9fOFP7r9Ol"

#define CRASH_TIMEOUT_MS 15000 // 15 Seconds timeout

#define MAX_RETRIES 2

static int64_t last_heartbeat = 0;

// Helper to get current time in milliseconds
static int64_t get_time_ms() {
    return esp_timer_get_time() / 1000;
}

// ---------------------------------------------------------
// 1. BUTTON LOGIC: Send "NEXT" Command (HTTP POST)
// ---------------------------------------------------------
void http_send_next_command(void) {
    char url[128];
    snprintf(url, sizeof(url), "http://%s:%d/next?token=%s", SERVER_IP, SERVER_PORT, DEVICE_TOKEN);

    esp_http_client_config_t config = {
        .url = url,
        .method = HTTP_METHOD_POST,
        .timeout_ms = 5000,
    };

    esp_http_client_handle_t client = esp_http_client_init(&config);
    
    // Perform the request
    esp_err_t err = esp_http_client_perform(client);

    if (err == ESP_OK) {
        int status_code = esp_http_client_get_status_code(client);
        ESP_LOGI(TAG, "Command sent! Status: %d", status_code);
    } else {
        ESP_LOGE(TAG, "Failed to send command: %s", esp_err_to_name(err));
        // Optional: Trigger AP mode here if button fails 3 times
    }

    esp_http_client_cleanup(client);
}

// ---------------------------------------------------------
// 2. SCREEN LOGIC: SSE Listener Task
// ---------------------------------------------------------

// Process a single line from SSE
void process_sse_line(char *line) {
    if (strstr(line, "PING") != NULL) {
        ESP_LOGI(TAG, "Heartbeat Received");
        last_heartbeat = get_time_ms();
        return;
    }
}

void sse_task(void *pvParameters) {
    char url[128];
    snprintf(url, sizeof(url), "http://%s:%d/events", SERVER_IP, SERVER_PORT);

    esp_http_client_config_t config = {
        .url = url,
        .timeout_ms = 10000,
        .keep_alive_enable = true,
    };

    esp_http_client_handle_t client = esp_http_client_init(&config);

    int retry_count = 0;

    // Initial Heartbeat Reset
    last_heartbeat = get_time_ms();

    while (1) {
        ESP_LOGI(TAG, "Connecting to SSE Server...");
        
        if (esp_http_client_open(client, 0) != ESP_OK) {
            ESP_LOGE(TAG, "Failed to open connection");
            
            retry_count++;
            if (retry_count >= MAX_RETRIES) {
                ESP_LOGE(TAG, ">>> MAX RETRIES REACHED. ENTERING AP MODE <<<");
                esp_http_client_cleanup(client);
                
                wifi_ap_mode();
                vTaskDelete(NULL);
                return;
            }
            goto retry;
        }

        if (esp_http_client_fetch_headers(client) < 0) {
            ESP_LOGE(TAG, "Failed to fetch headers");
            goto retry;
        }

        char buffer[1];
        char line_buffer[256];
        int line_pos = 0;

        while (1) {
            int read_len = esp_http_client_read(client, buffer, 1);

            if (read_len <= 0) {
                if (esp_http_client_is_complete_data_received(client)) {
                    ESP_LOGW(TAG, "Stream ended by server.");
                } else {
                    ESP_LOGE(TAG, "Stream read error/timeout.");
                }
                break;
            }

            if ((get_time_ms() - last_heartbeat) > CRASH_TIMEOUT_MS) {
                ESP_LOGE(TAG, "CRASH DETECTED: No Heartbeat for 15s!");
                esp_http_client_close(client);
                esp_http_client_cleanup(client);
                
                wifi_ap_mode(); 
                vTaskDelete(NULL);
            }

            if (buffer[0] == '\n') {
                line_buffer[line_pos] = '\0';
                if (line_pos > 5) {
                    process_sse_line(line_buffer);
                }
                line_pos = 0;
            } else if (buffer[0] != '\r') {
                if (line_pos < sizeof(line_buffer) - 1) {
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

void http_app_start_listener(void) {
    xTaskCreate(sse_task, "sse_task", 4096, NULL, 5, NULL);
}