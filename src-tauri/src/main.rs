// Sin ventana de consola en release (Windows).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    freeeditorstudio_lib::run()
}
