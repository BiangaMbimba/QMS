#include "wifi_set.h"
#include "esp_system.h"
#include "esp_wifi.h"
#include "esp_event.h"
#include "esp_log.h"
#include "lwip/err.h"
#include "lwip/sys.h"
#include "web_server.h"
#include "esp_event.h"
#include "memory.h"
#include "pins.h"
#include "websocket.h"

const char *TAG = "WIFI-QMS";

esp_netif_t *netif_sta = NULL;
esp_netif_t *netif_ap = NULL;

#define MAX_RETRY_STA 5

static int retry_wifi_sta = 0;

static void wifi_event_handler(void *arg, esp_event_base_t event_base, int32_t event_id, void *event_data)
{
    if (event_base == WIFI_EVENT)
    {
        switch (event_id)
        {
        case WIFI_EVENT_STA_START:
            esp_wifi_connect();
            set_leds_level(true);

            break;

        case WIFI_EVENT_STA_DISCONNECTED:
            if (retry_wifi_sta < MAX_RETRY_STA)
            {
                esp_wifi_connect();
                retry_wifi_sta++;
                ESP_LOGI(TAG, "Sttemp to connect ...");
            }
            else
            {
                ESP_LOGE(TAG, "Wifi sta failed to connect ...");
                wifi_ap_mode();
            }
            break;

        case WIFI_EVENT_AP_START:
            set_leds_level(false);
            break;

        case WIFI_EVENT_AP_STACONNECTED:
            break;
        }
    }

    if (event_base == IP_EVENT)
    {
        if (event_id == IP_EVENT_STA_GOT_IP)
        {
            retry_wifi_sta = 0;
            websocket_app_start();
        }
    }
}

void wifi_ap_mode()
{

    esp_wifi_stop();

    char pass[16] = {0};
    const char wifi_ssid[16] = "BUTTON QSM";

    nvs_get_info("device_info", "ap_pass", pass, sizeof(pass));

    wifi_config_t wifi_config = {
        .ap = {
            .ssid_len = strlen(wifi_ssid),
            .max_connection = 2,
            .authmode = WIFI_AUTH_WPA_WPA2_PSK}};

    strncpy((char *)wifi_config.ap.ssid, wifi_ssid, sizeof(wifi_config.ap.ssid));
    strncpy((char *)wifi_config.ap.password, pass, sizeof(wifi_config.ap.password));

    if (strlen(pass) == 0)
    {
        wifi_config.ap.authmode = WIFI_AUTH_OPEN;
    }

    esp_wifi_set_mode(WIFI_MODE_AP);
    esp_wifi_set_config(WIFI_IF_AP, &wifi_config);
    esp_wifi_start();

    start_web_server();
}

void wifi_sta_mode(char *wifi_ssid, char *pass)
{
    esp_wifi_stop();

    wifi_config_t wifi_config = {
        .sta = {
            .threshold.authmode = WIFI_AUTH_WPA2_PSK,
        }};

    strncpy((char *)wifi_config.sta.ssid, wifi_ssid, sizeof(wifi_config.sta.ssid));
    strncpy((char *)wifi_config.sta.password, pass, sizeof(wifi_config.sta.password));

    esp_wifi_set_mode(WIFI_MODE_STA);
    esp_wifi_set_config(WIFI_IF_STA, &wifi_config);

    esp_wifi_start();
}

void wifi_setup()
{

    netif_sta = esp_netif_create_default_wifi_sta();
    netif_ap  = esp_netif_create_default_wifi_ap();

    wifi_init_config_t config = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&config));

    esp_event_handler_instance_t instance_any_id;
    esp_event_handler_instance_t instance_got_ip;
    esp_event_handler_instance_register(WIFI_EVENT, ESP_EVENT_ANY_ID, &wifi_event_handler, NULL, &instance_any_id);
    esp_event_handler_instance_register(IP_EVENT, IP_EVENT_STA_GOT_IP, &wifi_event_handler, NULL, &instance_got_ip);

    bool has_valid_creds = false;
    char ssid[16] = {0};
    char pass[16] = {0};

    esp_err_t err_ssid = nvs_get_info("mqtt_info", "wifi_ssid", ssid, sizeof(ssid));
    esp_err_t err_pass = nvs_get_info("mqtt_info", "wifi_pass", pass, sizeof(pass));

    if (err_ssid == ESP_OK && strlen(ssid) > 0 && err_pass == ESP_OK && strlen(pass) > 0)
        has_valid_creds = true;

    if (has_valid_creds)
    {
        ESP_LOGI(TAG, "Identifiants trouvés (%s) ! Démarrage en mode Station.", ssid);
        wifi_sta_mode(ssid, pass);
    }
    else
        wifi_ap_mode();
}