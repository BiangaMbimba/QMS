#ifndef HTTP_APP_H
#define HTTP_APP_H

void http_app_start_listener(void); // Starts the SSE Task (For Screens)
void http_send_next_command(void);  // Sends POST request (For Buttons)

#endif