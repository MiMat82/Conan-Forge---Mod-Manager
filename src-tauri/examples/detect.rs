fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&conan_mod_manager_lib::detection::detect()).unwrap()
    );
}
