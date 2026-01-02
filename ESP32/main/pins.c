#include "pins.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

void set_leds_level(const bool level)
{
    gpio_set_level(GOOD_LED, level);
    gpio_set_level(CONF_LED, !level);
}

int button_pressed(gpio_num_t pin) {

    if (!gpio_get_level(pin))
    {
        vTaskDelay(pdMS_TO_TICKS(50));

        if (gpio_get_level(pin))
            return NO_PRESS_BIT;

        TickType_t start_time = xTaskGetTickCount();

        while (!gpio_get_level(pin)) {
            vTaskDelay(pdMS_TO_TICKS(10));

            if (xTaskGetTickCount() - start_time > pdMS_TO_TICKS(7000)) {
                return LONG_PRESS_BIT;
            }
        }
        return SHORT_PRESS_BIT;
    }
    return NO_PRESS_BIT;
}