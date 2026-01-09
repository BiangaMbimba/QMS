idf_component_register(
    SRCS "websocket.c" "pins.c" "wifi_set.c" "main.c" "web_server.c" "memory.c" "wifi_set.c" "pins.c"
    INCLUDE_DIRS "."
    PRIV_REQUIRES esp_wifi esp_driver_gpio esp_pm
    REQUIRES nvs_flash esp_http_server esp_http_client
)

target_add_binary_data(${COMPONENT_TARGET} "config_pages/index.html" TEXT)
