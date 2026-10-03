#[cfg(windows)]
fn main() {
    extern crate winres;
    let mut res = winres::WindowsResource::new();
    res.set_icon("icon.ico");
    res.compile().unwrap();
}

#[cfg(unix)]
fn main() {}
