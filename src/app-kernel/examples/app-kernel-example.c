#include "libapp_base.h"
#include "libapp_kernel.h"
#include <assert.h>
#include <malloc.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

const void *
main_module(const App *, const AppEvent *event, const void *context);
const char *get_event_name(AppEvent event);

const AppModule MOD_MAIN = main_module;
const char *NT_TEST = "Main::test";

int main(int argc, const char *argv[]) {
    const AppModule modules[] = {MOD_CMD};

    const App *app = app_new(modules, 1, argc, argv);

    if ((intptr_t *)app == &ERR_APP) {
        return EXIT_FAILURE;
    }

    app_add_notify_handler(app, NT_APP_RUN, MOD_MAIN);
    app_add_notify_handler(app, NT_TEST, MOD_MAIN);

    app_add_hook_handler(app, LOAD, "", MOD_MAIN);
    app_add_hook_handler(app, UNLOAD, "", MOD_MAIN);

    app_load_module(app, MOD_MAIN);

    app_set_default_cmd("test");

    const void *res = app_run(app, "Application Run Context");

    malloc_stats();
    app_free(app);
    malloc_stats();

    if ((intptr_t *)res == &ERR_APP) {
        return EXIT_FAILURE;
    }

    return EXIT_SUCCESS;
}

const void *
main_module(const App *app, const AppEvent *event, const void *context) {
    char buf[100] = "";
    const char *event_name = get_event_name(*event);

    sprintf(
        buf, "Catched event: %s context: %lx", event_name,
        (unsigned long)context
    );
    log_msg(DEBUG, __FUNCTION__, buf);

    if (*event == LOAD && context == MOD_MAIN) {
        char *c = "Main module loaded";
        app_notify(app, NT_TEST, MOD_MAIN, c);
    }

    if (*event == NOTIFY) {
        char *notify = app_get_event_notify(event);

        sprintf(buf, "Notify: '%s' Context: '%s'", notify, (char *)context);
        log_msg(INFO, __FUNCTION__, buf);

        if (strcmp(notify, NT_APP_RUN) == 0) {
            app_set_event_handled(event, true);

            for (int i = 0; i < 10; i++) {
                app_notify(app, NT_TEST, MOD_MAIN, "Example context");
            }
        }

        free(notify);
    }

    if (*event == HOOK) {
        log_msg(DEBUG, __FUNCTION__, "Hook handled");
    }

    if (*event == UNLOAD && context == MOD_MAIN) {
        app_notify(app, NT_TEST, MOD_MAIN, "Main module unloaded");
    }

    return NULL;
    // OR
    // return app_error("Error from C module");
}

const char *get_event_name(AppEvent event) {
    switch (event) {
    case LOAD:
        return "LOAD";
    case NOTIFY:
        return "NOTIFY";
    case HOOK:
        return "HOOK";
    case UNLOAD:
        return "UNLOAD";
    case META:
        return "META";
    default:
        return "Unknown";
    };
}
