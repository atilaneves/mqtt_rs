use std::process::Command;

fn main() {
    Command::new("dmd").args(&["-lib", "-Id", "-fPIC",
                               "d/mqttd/broker.d", "d/mqttd/server.d","d/mqttd/stream.d","d/mqttd/message.d",
                               "d/cerealed/cereal.d", "d/cerealed/package.d", "d/cerealed/cerealiser.d", "d/cerealed/decerealiser.d",
                               "d/cerealed/attrs.d", "d/cerealed/scopebuffer.d", "d/cerealed/range.d",
                               "d/impl.d",
                               "-oftarget/libmqttd.a"]).status().unwrap();
    println!("cargo:rustc-link-lib=static=mqttd");
    println!("cargo:rustc-link-lib=dylib=phobos2");
    println!("cargo:rustc-link-search=native=target");
}
