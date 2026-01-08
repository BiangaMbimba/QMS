#include "websocket.h"
#include "esp_websocket_client.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_system.h"
#include "esp_event.h"
#include "esp_log.h"
#include "wifi_set.h"

static const char *TAG = "WS_CLIENT";
static int retry_counter = 0;
static const int MAX_RETRIES = 5;

esp_websocket_client_handle_t client;

static void websocket_event_handler(void *arg, esp_event_base_t event_base, int32_t event_id, void *event_data)
{
    esp_websocket_event_data_t *data = (esp_websocket_event_data_t *)event_data;

    switch (event_id)
    {
    case WEBSOCKET_EVENT_CONNECTED:
        ESP_LOGI(TAG, "WEBSOCKET_EVENT_CONNECTED");
        
        retry_counter = 0;
        // Optional: Send a message immediately upon connection
        char *msg = "Hello from ESP32";
        esp_websocket_client_send_text(data->client, msg, strlen(msg), portMAX_DELAY);

        break;
    
    case WEBSOCKET_EVENT_DISCONNECTED:
        ESP_LOGI(TAG, "WEBSOCKET_EVENT_DISCONNECTED");

        if (retry_counter < MAX_RETRIES) {
            retry_counter ++;
            ESP_LOGW(TAG, "Retrying connection ... Attemp %d", retry_counter);
            esp_websocket_client_start(data->client);
        } else {
            ESP_LOGE(TAG,"Max retries reached. Stopping client");
            wifi_ap_mode();
        }

        break;

    case WEBSOCKET_EVENT_DATA:
        ESP_LOGI(TAG, "WEBSOCKET_EVENT_DATA");
        ESP_LOGI(TAG, "Received opcode=%d", data->op_code);
        
        if (data->op_code == WS_TRANSPORT_OPCODES_TEXT) {
            ESP_LOGW(TAG, "Received=%.*s", data->data_len, (char *)data->data_ptr);
        }
        break;

    case WEBSOCKET_EVENT_ERROR:
        ESP_LOGE(TAG, "WEBSOCKET_EVENT_ERROR");
        break;
    }
}

void websocket_app_start(void)
{
    const char *ip_server = "192.168.0.199";
    const char *token = "0hW1DI9fOFP7r9Ol";

    char uri_buffer[128];

    snprintf(uri_buffer, sizeof(uri_buffer), "ws://%s:8765/?token=%s", ip_server, token);

    esp_websocket_client_config_t websocket_cfg = {
        .uri = uri_buffer,
        .disable_auto_reconnect = true,
    };

    client = esp_websocket_client_init(&websocket_cfg);
    esp_websocket_register_events(client, WEBSOCKET_EVENT_ANY, websocket_event_handler, (void *)client);
    esp_websocket_client_start(client);
}

void websocket_send_message(const char *msg) {
    esp_websocket_client_send_text(client, msg, strlen(msg), portMAX_DELAY);
} 