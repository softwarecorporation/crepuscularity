/* Lifetime probe only: all pointers refer to static storage, never freed or
 * dereferenced. Even the unguarded baseline cannot execute a stale pointer. */
static int session;
static int calls;
static int frees;
static char rendered[] = "{\"root\":[{}]}";
static char dispatched[] = "{\"kind\":\"event\"}";

int lifetime_native_calls(void) { return calls; }
int lifetime_free_calls(void) { return frees; }
void *crepus_session_new(void) { ++calls; return &session; }
void crepus_session_free(void *pointer) { (void)pointer; ++calls; ++frees; }
int crepus_session_set_template_string(void *pointer, const char *source, const char *base_dir) {
    (void)pointer; (void)source; (void)base_dir; ++calls; return 0;
}
int crepus_session_set_context_json(void *pointer, const char *json) {
    (void)pointer; (void)json; ++calls; return 0;
}
int crepus_session_apply_context_patch_json(void *pointer, const char *json) {
    (void)pointer; (void)json; ++calls; return 0;
}
void *crepus_session_render_ir_json(void *pointer) {
    (void)pointer; ++calls; return rendered;
}
void *crepus_session_dispatch_event_json(void *pointer, const char *json) {
    (void)pointer; (void)json; ++calls; return dispatched;
}
void *crepus_session_take_last_error(void *pointer) {
    (void)pointer; ++calls; return 0;
}
void crepus_string_free(void *pointer) { (void)pointer; ++calls; }
