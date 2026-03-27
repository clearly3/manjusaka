fn main() {
    /*
    let mut config = prost_build::Config::new();
            config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
            config.type_attribute("nps.Agent", "#[derive(poem_openapi::Object)]");
            config.out_dir("src/protos");
            config.compile_protos(&["../protos/nps.proto"], &["../protos/"]).unwrap();
            config.compile_protos(&["../protos/npc2.proto"], &["../protos/"]).unwrap();
    */
    static_vcruntime::metabuild();
}
