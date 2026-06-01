#include "esp_wifi.h"

wifi_init_config_t orion_wifi_init_config_default(void) {
    wifi_init_config_t config = WIFI_INIT_CONFIG_DEFAULT();
    return config;
}
