#include "web_server.h"
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_system.h"
#include "esp_event.h"
#include "esp_log.h"
#include "nvs_flash.h"
#include "nvs.h"
#include "esp_http_server.h"
#include "memory.h"
#include "esp_random.h"
#include "cJSON.h"

static const char *TAG = "WEBSERVER-LOGS";
static const char *default_admin = "admin";
static const char *default_password = "admin123";

static char current_session_id[33] = {0}; // Stores the active token
extern const uint8_t index_html_start[] asm("_binary_index_html_start");
extern const uint8_t index_html_end[] asm("_binary_index_html_end");

/**
 * @brief To validate the session
 */
bool is_session_valid(httpd_req_t *req)
{
    char buf[100];

    if (httpd_req_get_hdr_value_str(req, "Cookie", buf, sizeof(buf)) == ESP_OK)
    {

        if (strstr(buf, current_session_id) != NULL)
        {
            if (strlen(current_session_id) > 0)
            {
                return true;
            }
        }
    }
    return false;
}

/**
 * @brief To compare and store data in case of difference
 */
esp_err_t cmp_and_store(const char *type, const char *key, const char *info, const char *memory_info)
{
    if (strcmp(info, memory_info) != 0)
    {
        if (nvs_set_info(type, key, info) != ESP_OK)
        {
            ESP_LOGE(TAG, "Failed to store %s ...", key);
            return ESP_FAIL;
        }
    }

    return ESP_OK;
}

/**
 * @brief To extract json data
 *
 * @param
 *    - req -> The request
 *    - output -> Map variable of cJSON type
 */
static esp_err_t extract_data(httpd_req_t *req, cJSON **output)
{
    char *buf;
    size_t content_len = req->content_len;

    if (content_len <= 0)
        return ESP_FAIL;

    buf = malloc(content_len + 1);
    if (buf == NULL)
    {
        ESP_LOGE(TAG, "Failed to allocate memory for request buffer");
        return ESP_ERR_NO_MEM;
    }

    int ret = httpd_req_recv(req, buf, content_len);
    if (ret <= 0)
    {
        if (ret == HTTPD_SOCK_ERR_TIMEOUT)
        {
            httpd_resp_send_408(req);
        }
        free(buf);
        return ESP_FAIL;
    }

    buf[ret] = '\0';

    ESP_LOGI(TAG, "Received POST data: %s", buf);

    *output = cJSON_Parse(buf);

    free(buf);

    if (*output == NULL)
    {
        ESP_LOGE(TAG, "Failed to parse JSON");
        return ESP_FAIL;
    }

    return ESP_OK;
}

/**
 * @brief Send the html cofig page
 */
static esp_err_t root_get_handler(httpd_req_t *req)
{

    ssize_t html_len = index_html_end - index_html_start;

    httpd_resp_send(req, (const char *)index_html_start, html_len);
    return ESP_OK;
}

/**
 * @brief To send device info in to the config page
 */
static esp_err_t device_info_get_handler(httpd_req_t *req)
{

    char dev_pass[16] = {0};
    char ap_pass[16] = {0};

    if (nvs_get_info("device_info", "dev_pass", dev_pass, sizeof(dev_pass)) != ESP_OK || nvs_get_info("device_info", "ap_pass", ap_pass, sizeof(ap_pass)) == ESP_FAIL)
    {
        ESP_LOGE(TAG, "Failed to read the device info ...");
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }

    cJSON *data = cJSON_CreateObject();

    cJSON_AddStringToObject(data, "device_password", dev_pass);

    if (strlen(ap_pass))
        cJSON_AddStringToObject(data, "ap_password", ap_pass);

    char *json_string = cJSON_PrintUnformatted(data);

    httpd_resp_set_type(req, "application/json");
    httpd_resp_send(req, json_string, strlen(json_string));

    cJSON_free(json_string);
    cJSON_Delete(data);

    return ESP_OK;
}

/**
 * @brief To send mqtt info in to the config page
 */
static esp_err_t share_info_get_handler(httpd_req_t *req)
{

    char mqtt_user[16] = {0};
    char mqtt_pass[16] = {0};
    char broker_ip[16] = {0};
    char wifi_ssid[16] = {0};
    char wifi_pass[16] = {0};

    if (nvs_get_info("mqtt_info", "mqtt_name", mqtt_user, sizeof(mqtt_user)) == ESP_FAIL || nvs_get_info("mqtt_info", "mqtt_pass", mqtt_pass, sizeof(mqtt_pass)) == ESP_FAIL || nvs_get_info("mqtt_info", "broker_ip", broker_ip, sizeof(broker_ip)) == ESP_FAIL || nvs_get_info("mqtt_info", "wifi_ssid", wifi_ssid, sizeof(wifi_ssid)) == ESP_FAIL || nvs_get_info("mqtt_info", "wifi_pass", wifi_pass, sizeof(wifi_pass)) == ESP_FAIL)
    {
        ESP_LOGE(TAG, "Failed to read the device info ...");
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }

    cJSON *data = cJSON_CreateObject();

    if (strlen(mqtt_user))
        cJSON_AddStringToObject(data, "mqtt_name", mqtt_user);

    if (strlen(mqtt_pass))
        cJSON_AddStringToObject(data, "mqtt_pass", mqtt_pass);

    if (strlen(broker_ip))
        cJSON_AddStringToObject(data, "broker_ip", broker_ip);

    if (strlen(wifi_ssid))
        cJSON_AddStringToObject(data, "wifi_ssid", wifi_ssid);

    if (strlen(wifi_pass))
        cJSON_AddStringToObject(data, "wifi_pass", wifi_pass);

    char *json_string = cJSON_PrintUnformatted(data);

    httpd_resp_set_type(req, "application/json");
    httpd_resp_send(req, json_string, strlen(json_string));

    cJSON_free(json_string);
    cJSON_Delete(data);

    return ESP_OK;
}

/**
 * @brief To authenticate the user, generate also a session id
 */
static esp_err_t login_post_handler(httpd_req_t *req)
{
    cJSON *data = NULL;
    char cookie_header[64];

    if (extract_data(req, &data) != ESP_OK)
    {
        ESP_LOGE(TAG, "Fail to read json data ..........");
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }

    cJSON *user_item = cJSON_GetObjectItemCaseSensitive(data, "username");
    cJSON *pass_item = cJSON_GetObjectItemCaseSensitive(data, "password");

    ESP_LOGI(TAG, "Got -> username: %s and password %s",
             user_item->valuestring,
             pass_item->valuestring);

    // get data in nvs
    char device_password[16] = {0};

    esp_err_t devide_err = nvs_get_info("device_info", "dev_pass", device_password, sizeof(device_password));

    if (devide_err == ESP_OK || devide_err == ESP_ERR_NVS_NOT_FOUND)
    {

        uint32_t random_num = esp_random();
        snprintf(current_session_id, sizeof(current_session_id), "%lu", (unsigned long)random_num);

        if (strlen(device_password) > 0)
        {
            if (strcmp(user_item->valuestring, default_admin) != 0 || strcmp(device_password, pass_item->valuestring) != 0)
            {
                httpd_resp_send_err(req, HTTPD_404_NOT_FOUND, "Either username or password is not correct!");
                return ESP_FAIL;
            }
        }
        else
        {
            if (devide_err == ESP_ERR_NVS_NOT_FOUND)
            {
                nvs_set_info("device_info", "dev_pass", default_password);

                if (strcmp(user_item->valuestring, "admin") != 0 || strcmp(pass_item->valuestring, default_password) != 0)
                {
                    httpd_resp_send_err(req, HTTPD_404_NOT_FOUND, "Either username or password is not correct!");
                    return ESP_FAIL;
                }
            }
        }

        cJSON_Delete(data);

        snprintf(cookie_header, sizeof(cookie_header), "SESSIONID=%s; Path=/; Max-Age=600", current_session_id);

        httpd_resp_set_hdr(req, "Set-Cookie", cookie_header);
        httpd_resp_send(req, "OK", HTTPD_RESP_USE_STRLEN);
        return ESP_OK;
    }
    else
    {
        cJSON_Delete(data);
        ESP_LOGE(TAG, "Auth error, Nvs inaccessible ...");
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }
}

/**
 * @brief To change device information
 *  - Auth password
 *  - AP password
 */
static esp_err_t device_save_post_handler(httpd_req_t *req)
{

    bool isSmtChange = false;

    if (!is_session_valid(req))
    {
        ESP_LOGW(TAG, "Unauthorized access attempt!");
        httpd_resp_send_err(req, HTTPD_401_UNAUTHORIZED, "Session Expired");
        return ESP_FAIL;
    }

    cJSON *data = NULL;

    if (extract_data(req, &data) != ESP_OK)
    {
        ESP_LOGE(TAG, "Fail to read json data ..........");
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }

    cJSON *dev_pass = cJSON_GetObjectItemCaseSensitive(data, "dev_pass");
    cJSON *ap_pass = cJSON_GetObjectItemCaseSensitive(data, "ap_pass");

    char m_dev_pass[16] = {0};
    char m_ap_pass[16] = {0};

    if (nvs_get_info("device_info", "dev_pass", m_dev_pass, sizeof(m_dev_pass)) != ESP_OK)
    {
        ESP_LOGE(TAG, "Failed to read the device password");
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }

    if (nvs_get_info("device_info", "ap_pass", m_ap_pass, sizeof(m_ap_pass)) == ESP_FAIL)
    {
        ESP_LOGE(TAG, "Failed to read the ap password");
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }

    if (cmp_and_store("device_info", "dev_pass", dev_pass->valuestring, m_dev_pass) != ESP_OK)
    {
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }
    else
        isSmtChange = true;

    if (cmp_and_store("device_info", "ap_pass", ap_pass->valuestring, m_ap_pass) != ESP_OK)
    {
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }
    else
        isSmtChange = true;

    ESP_LOGI(TAG, "Got -> SSID: %s and PASSWORD %s", dev_pass->valuestring, ap_pass->valuestring);

    cJSON_Delete(data);

    httpd_resp_send(req, "Saved and reboot ...", HTTPD_RESP_USE_STRLEN);
    vTaskDelay(1000 / portTICK_PERIOD_MS);

    if (isSmtChange)
    {
        esp_restart();
        isSmtChange = false;
    }

    return ESP_OK;
}

/**
 * @brief To change MQTT info
 */
esp_err_t share_save_post_handler(httpd_req_t *req)
{

    bool isSmtChange = false;

    if (!is_session_valid(req))
    {
        ESP_LOGW(TAG, "Unauthorized access attempt!");
        httpd_resp_send_err(req, HTTPD_401_UNAUTHORIZED, "Session Expired");
        return ESP_FAIL;
    }

    cJSON *data = NULL;

    if (extract_data(req, &data) != ESP_OK)
    {
        ESP_LOGE(TAG, "Fail to read json data ..........");
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }

    cJSON *mqtt_user = cJSON_GetObjectItemCaseSensitive(data, "mqtt_user");
    cJSON *mqtt_pass = cJSON_GetObjectItemCaseSensitive(data, "mqtt_pass");
    cJSON *broker_ip = cJSON_GetObjectItemCaseSensitive(data, "broker_ip");
    cJSON *wifi_ssid = cJSON_GetObjectItemCaseSensitive(data, "ssid");
    cJSON *wifi_pass = cJSON_GetObjectItemCaseSensitive(data, "pass");

    char m_mqtt_user[16] = {0};
    char m_mqtt_pass[16] = {0};
    char m_broker_ip[16] = {0};
    char m_wifi_ssid[16] = {0};
    char m_wifi_pass[16] = {0};

    if (
        nvs_get_info("mqtt_info", "mqtt_name", m_mqtt_user, sizeof(m_mqtt_user)) == ESP_FAIL || nvs_get_info("mqtt_info", "mqtt_pass", m_mqtt_pass, sizeof(m_mqtt_pass)) == ESP_FAIL || nvs_get_info("mqtt_info", "broker_ip", m_broker_ip, sizeof(m_broker_ip)) == ESP_FAIL || nvs_get_info("mqtt_info", "wifi_ssid", m_wifi_ssid, sizeof(m_wifi_ssid)) == ESP_FAIL || nvs_get_info("mqtt_info", "wifi_pass", m_wifi_pass, sizeof(m_wifi_pass)) == ESP_FAIL)
    {
        ESP_LOGE(TAG, "Failed to read mqtt info ");
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }

    if (cmp_and_store("mqtt_info", "mqtt_name", mqtt_user->valuestring, m_mqtt_user) != ESP_OK)
    {
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }
    else
        isSmtChange = true;

    if (cmp_and_store("mqtt_info", "mqtt_pass", mqtt_pass->valuestring, m_mqtt_pass) != ESP_OK)
    {
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }
    else
        isSmtChange = true;

    if (cmp_and_store("mqtt_info", "broker_ip", broker_ip->valuestring, m_broker_ip) != ESP_OK)
    {
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }
    else
        isSmtChange = true;

    if (cmp_and_store("mqtt_info", "wifi_pass", wifi_pass->valuestring, m_wifi_pass) != ESP_OK)
    {
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }
    else
        isSmtChange = true;

    if (cmp_and_store("mqtt_info", "wifi_ssid", wifi_ssid->valuestring, m_wifi_ssid) != ESP_OK)
    {
        httpd_resp_send_500(req);
        return ESP_FAIL;
    }
    else
        isSmtChange = true;

    cJSON_Delete(data);

    httpd_resp_send(req, "Saved and reboot ...", HTTPD_RESP_USE_STRLEN);
    vTaskDelay(1000 / portTICK_PERIOD_MS);

    if (isSmtChange)
    {
        esp_restart();
        isSmtChange = false;
    }

    return ESP_OK;
}

/**
 * @brief To set different uris (get and post)
 */
void start_web_server(void)
{
    httpd_config_t config = HTTPD_DEFAULT_CONFIG();
    httpd_handle_t server = NULL;

    if (httpd_start(&server, &config) == ESP_OK)
    {

        // ********* GET requests handler ***************

        httpd_uri_t uri_root = {
            .uri = "/",
            .method = HTTP_GET,
            .handler = root_get_handler,
            .user_ctx = NULL};

        httpd_register_uri_handler(server, &uri_root);

        httpd_uri_t uri_get_device_info = {
            .uri = "/dev_info",
            .method = HTTP_GET,
            .handler = device_info_get_handler,
            .user_ctx = NULL};

        httpd_register_uri_handler(server, &uri_get_device_info);

        httpd_uri_t uri_get_share_info = {
            .uri = "/share_info",
            .method = HTTP_GET,
            .handler = share_info_get_handler,
            .user_ctx = NULL};

        httpd_register_uri_handler(server, &uri_get_share_info);

        // ********* POST requests handler ***************

        httpd_uri_t uri_save_device_info = {
            .uri = "/save_device",
            .method = HTTP_POST,
            .handler = device_save_post_handler,
            .user_ctx = NULL};

        httpd_register_uri_handler(server, &uri_save_device_info);

        httpd_uri_t uri_save_share_info = {
            .uri = "/save_share",
            .method = HTTP_POST,
            .handler = share_save_post_handler,
            .user_ctx = NULL};

        httpd_register_uri_handler(server, &uri_save_share_info);

        httpd_uri_t uri_login = {
            .uri = "/login",
            .method = HTTP_POST,
            .handler = login_post_handler,
            .user_ctx = NULL};

        httpd_register_uri_handler(server, &uri_login);
    }
}