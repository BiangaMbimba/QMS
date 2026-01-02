#ifndef __PINS_QMS
#define __PINS_QMS

#include "esp_system.h"
#include "driver/gpio.h"

#define GOOD_LED 41
#define CONF_LED 40
#define PUSH_BUTTON 39
#define POWER_BUTTON 5

#define LONG_PRESS_BIT BIT2
#define SHORT_PRESS_BIT BIT1
#define NO_PRESS_BIT BIT3

/**
 * @brief To set on/off the config and working leds
 * 
 * @param level
 *      - true: good led on
 *      - false: config led on 
 */
void set_leds_level(const bool level);

/**
 * @brief To verify the press duration
 * 
 * @param pin
 *      the pullup pin
 */
int button_pressed(gpio_num_t pin);

#endif