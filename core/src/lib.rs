#[macro_use]
extern crate napi_derive;

/// Funcao-sentinela da Fase 0: prova que a ponte Electron <-> Rust esta viva.
#[napi]
pub fn hello(name: String) -> String {
    format!(
        "Hello from Rust (openshoot-core v{})! Ponte Electron <-> Rust OK. Bem-vindo, {name}!",
        env!("CARGO_PKG_VERSION")
    )
}

#[napi(js_name = "coreVersion")]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Soma basica para validar transporte de numeros pela ponte NAPI.
#[napi]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    #[test]
    fn hello_contains_name() {
        let msg = super::hello("Test".to_string());
        assert!(msg.contains("Test"));
        assert!(msg.contains("OK"));
    }

    #[test]
    fn add_works() {
        assert_eq!(super::add(2, 3), 5);
    }

    #[test]
    fn version_non_empty() {
        assert!(!super::core_version().is_empty());
    }
}
