#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>
#include <unistd.h>
#include <string.h>
#include <pthread.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <assert.h>
#include <signal.h>
#include <time.h>
#include <errno.h>

#include "api.hpp"

// Mock printer server
void* mock_printer_server(void* arg) {
    int server_fd;
    struct sockaddr_in address;
    int opt = 1;
    fd_set readfds;
    struct timeval tv;

    if ((server_fd = socket(AF_INET, SOCK_STREAM, 0)) == 0) {
        perror("socket failed");
        exit(EXIT_FAILURE);
    }

    if (setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR | SO_REUSEPORT, &opt, sizeof(opt))) {
        perror("setsockopt");
        exit(EXIT_FAILURE);
    }
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = INADDR_ANY;
    address.sin_port = htons(8888);

    if (bind(server_fd, (struct sockaddr *)&address, sizeof(address)) < 0) {
        perror("bind failed");
        exit(EXIT_FAILURE);
    }
    if (listen(server_fd, 3) < 0) {
        perror("listen");
        exit(EXIT_FAILURE);
    }

    FD_ZERO(&readfds);
    FD_SET(server_fd, &readfds);
    tv.tv_sec = 5;
    tv.tv_usec = 0;

    int activity = select(server_fd + 1, &readfds, NULL, NULL, &tv);

    if ((activity < 0) && (errno != EINTR)) {
        printf("select error");
    }

    if (activity == 0) {
        printf("Mock printer server timed out.\n");
        close(server_fd);
        return NULL;
    }

    int new_socket;
    int addrlen = sizeof(address);
    if ((new_socket = accept(server_fd, (struct sockaddr *)&address, (socklen_t*)&addrlen)) < 0) {
        perror("accept");
        exit(EXIT_FAILURE);
    }

    // Handle client connection
    char buffer[1024] = {0};
    read(new_socket, buffer, 1024);
    printf("Mock printer received: %s\n", buffer);
    const char* response = "{\"command\":\"get_version_rsp\",\"sequence_id\":0,\"ret_code\":0}";
    send(new_socket, response, strlen(response), 0);
    close(new_socket);
    close(server_fd);
    return NULL;
}

static int printer_available = 0;
static int local_connected = 0;
static int message_received = 0;

void on_msg_arrived(const std::string &json) {
    printf("on_msg_arrived: %s\n", json.c_str());
    printer_available = 1;
}

void on_local_connect(int status, const std::string &device_id, const std::string &msg) {
    printf("on_local_connect: status=%d, device_id=%s, msg=%s\n", status, device_id.c_str(), msg.c_str());
    local_connected = 1;
}

void on_mqtt_message(const std::string &device_id, const std::string &message) {
    printf("on_mqtt_message: device_id=%s, message=%s\n", device_id.c_str(), message.c_str());
    message_received = 1;
}

void handle_alarm(int sig) {
    printf("Test timed out.\n");
    exit(1);
}

int main() {
    signal(SIGALRM, handle_alarm);
    alarm(30);
    void* handle = dlopen("bambu-farm-client/target/debug/libbambu_farm_client.so", RTLD_LAZY);
    if (!handle) {
        fprintf(stderr, "Error: %s\n", dlerror());
        return 1;
    }

    void* (*create_agent)() = (void* (*)())dlsym(handle, "bambu_network_create_agent");
    int (*set_on_ssdp_msg_fn)(void*, OnMsgArrivedFn) = (int (*)(void*, OnMsgArrivedFn))dlsym(handle, "bambu_network_set_on_ssdp_msg_fn");
    int (*set_on_local_connect_fn)(void*, OnLocalConnectedFn) = (int (*)(void*, OnLocalConnectedFn))dlsym(handle, "bambu_network_set_on_local_connect_fn");
    int (*set_on_local_message_fn)(void*, OnMessageFn) = (int (*)(void*, OnMessageFn))dlsym(handle, "bambu_network_set_on_local_message_fn");
    int (*connect_printer)(void*, std::string, std::string, std::string, std::string, bool) = (int (*)(void*, std::string, std::string, std::string, std::string, bool))dlsym(handle, "bambu_network_connect_printer");
    int (*send_message_to_printer)(void*, std::string, std::string, int) = (int (*)(void*, std::string, std::string, int))dlsym(handle, "bambu_network_send_message_to_printer");
    bool (*start_discovery)(void*, bool, bool) = (bool (*)(void*, bool, bool))dlsym(handle, "bambu_network_start_discovery");


    void* agent = create_agent();
    set_on_ssdp_msg_fn(agent, on_msg_arrived);
    set_on_local_connect_fn(agent, on_local_connect);
    set_on_local_message_fn(agent, on_mqtt_message);

    pthread_t mock_printer_thread;
    pthread_create(&mock_printer_thread, NULL, mock_printer_server, NULL);

    printf("Starting discovery...\n");
    start_discovery(agent, true, true);
    sleep(2);
    printf("Checking if printer is available...\n");
    assert(printer_available);

    printf("Connecting to printer...\n");
    connect_printer(agent, "123", "127.0.0.1", "bblp", "123456", false);
    sleep(1);
    printf("Checking if connected...\n");
    assert(local_connected);
    
    printf("Sending message to printer...\n");
    send_message_to_printer(agent, "123", "{\"command\":\"get_version\"}", 0);
    sleep(1);
    printf("Checking if message was received...\n");
    assert(message_received);

    struct timespec ts;
    if (clock_gettime(CLOCK_REALTIME, &ts) == -1) {
        perror("clock_gettime");
        exit(EXIT_FAILURE);
    }
    ts.tv_sec += 5;
    int s = pthread_timedjoin_np(mock_printer_thread, NULL, &ts);
    if (s != 0) {
        printf("pthread_timedjoin_np error %d\n", s);
    }

    dlclose(handle);
    return 0;
}

