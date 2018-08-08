macro_rules! to_stdout {
    ($($arg:tt)*) => (
        if writeln!(&mut ::std::io::stdout(), $($arg)*).is_ok() {
            ::std::io::stdout().flush().is_ok();
        });
}

macro_rules! to_stderr {
    ($($arg:tt)*) => (
        if writeln!(&mut ::std::io::stderr(), $($arg)*).is_ok()
        {
            ::std::io::stdout().flush().is_ok();
        });
}
