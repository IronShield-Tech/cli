
/// Macro for verbose printing that only prints if verbose mode is enabled.
///
/// # Example
/// ```
/// verbose_println!(config, "Simple message")
/// verbose_println!(config, "Formatted message: {}", value)
/// verbose_println!(config, "Multiple values: {} and {}", val1, val2)
/// ```
#[macro_export]
macro_rules! verbose_println {
    ($config:expr, $($arg:tt)*) => {
        if $config.verbose {
            println!($($arg)*);
        }
    };
}

/// Macro for verbose printing without newline that only prints if verbose mode is enabled.
///
/// # Example
/// ```
/// verbose_print!(config, "Message without newline")
/// verbose_print!(config, "Progress: {}", percentage)
/// ```
#[macro_export]
macro_rules! verbose_print {
    ($config:expr, $($arg:tt)*) => {
        if $config.verbose {
            print!($($arg)*);
            use std::io::{self, Write};
            let _ = io::stdout().flush(); // Ensure immediate output.
        }
    };
}

/// Macro for verbose *logging* with a new line that prints only if
/// verbose mode is enabled.
///
/// # Example
/// ```
/// verbose_log!(config, info, "Information")
/// verbose_log!(config, success, "Success")
/// ```
#[macro_export]
macro_rules! verbose_log {
    ($config:expr, compute, $($arg:tt)*) => {
        if $config.verbose {
            println!("COMPUTE: {}", format_args!($($arg)*));
        }
    };
    ($config:expr, error, $($arg:tt)*) => {
        if $config.verbose {
            println!("ERROR: {}", format_args!($($arg)*));
        }
    };
    ($config:expr, info, $($arg:tt)*) => {
        if $config.verbose {
            println!("INFO: {}", format_args!($($arg)*));
        }
    };
    ($config:expr, receive, $($arg:tt)*) => {
        if $config.verbose {
            println!("RECEIVE: {}", format_args!($($arg)*));
        }
    };
    ($config:expr, success, $($arg:tt)*) => {
        if $config.verbose {
            println!("SUCCESS: {}", format_args!($($arg)*));
        }
    };
    ($config:expr, submit, $($arg:tt)*) => {
        if $config.verbose {
            println!("SUBMIT: {}", format_args!($($arg)*));
        }
    };
    ($config:expr, network, $($arg:tt)*) => {
        if $config.verbose {
            println!("NETWORK: {}", format_args!($($arg)*));
        }
    };
    ($config:expr, timing, $($arg:tt)*) => {
        if $config.verbose {
            println!("TIMING: {}", format_args!($($arg)*));
        }
    };
    ($config:expr, warning, $($arg:tt)*) => {
        if $config.verbose {
            println!("WARNING: {}", format_args!($($arg)*));
        }
    };
}

/// Macro for displaying key-value pairs in a formatted way.
///
/// # Example
/// ```
/// verbose_kv!(config, "Endpoint", endpoint_url);
/// verbose_kv!(config, "Threads", num_threads);
/// verbose_kv!(config, "Duration", format!("{:?}", duration));
/// ```
#[macro_export]
macro_rules! verbose_kv {
    ($config:expr, $key:expr, $value:expr) => {
        if $config.verbose {
            println!("{}: {}", $key, $value);
        }
    };
}

/// Macro for displaying section headers in verbose output.
///
/// # Example
/// ```
/// verbose_section!(config, "Challenge Solving");
/// verbose_section!(config, "Network Communication");
/// ```
#[macro_export]
macro_rules! verbose_section {
    ($config:expr, $($arg:tt)*) => {
        if $config.verbose {
            println!("\n🔸 {}", format_args!($($arg)*));
            println!("{}", "─".repeat(40));
        }
    };
}
