fn main() {
    use png::Decoder;
    use std::{fs::File, io::Write};

    let ablavema32_file = File::open("extra/logo/ablavema32.png").unwrap();
    let decoder = Decoder::new(ablavema32_file);
    let mut reader = decoder.read_info().unwrap();
    let buffer_size = reader.output_buffer_size();
    let mut buf = vec![0; buffer_size];
    let info = reader.next_frame(&mut buf).unwrap();
    let mut iced_icon_data_file = File::create("extra/temp/iced_icon_data").unwrap();

    iced_icon_data_file.write_all(&buf).unwrap();
    println!("cargo:rustc-env=ICED_ICON_WIDTH={}", info.width);
    println!("cargo:rustc-env=ICED_ICON_HEIGHT={}", info.height);

    #[cfg(target_os = "windows")]
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("extra/windows/ablavema.ico");
        resource.set("FileDescription", "Ablavema");
        resource.set("ProductName", "Ablavema");
        resource.set("OriginalFilename", "ablavema.exe");
        resource.compile().unwrap();
    }
}
