use std::path::PathBuf;

fn main() {
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // cardinal-rs/
        .parent().unwrap()  // Cardinal/
        .to_path_buf();

    let plugins_dir = cardinal_root.join("plugins");
    let plugin_dir = plugins_dir.join("cf");

    if !plugin_dir.exists() {
        eprintln!("Plugin cf not found (submodule not initialized?), skipping");
        return;
    }

    let rack_dir = cardinal_root.join("src/Rack");

    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").warnings(false)
        .define("ARCH_X64", None)
        .define("ARCH_LIN", None)
        .define("BUILDING_PLUGIN_MODULES", None);

    // Rack includes
    for dir in &[
        rack_dir.join("include"),
        rack_dir.join("dep/glfw/include"),
        rack_dir.join("dep/nanovg/src"),
        rack_dir.join("dep/nanosvg/src"),
        rack_dir.join("dep/oui-blendish"),
        rack_dir.join("dep/osdialog"),
        rack_dir.join("dep/simde"),
        rack_dir.join("dep/filesystem/include"),
        rack_dir.join("dep/tinyexpr"),
        rack_dir.join("dep/pffft"),
        cardinal_root.join("include"),
        cardinal_root.join("plugins"),
        cardinal_root.join("dpf/distrho"),
    ] {
        build.include(dir);
    }

    // Plugin-local includes — recursively add src/ and dep subdirs
    fn add_dirs(build: &mut cc::Build, dir: &std::path::Path, depth: u32) {
        if depth > 4 || !dir.exists() { return; }
        build.include(dir);
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                if e.path().is_dir() {
                    add_dirs(build, &e.path(), depth + 1);
                }
            }
        }
    }
    add_dirs(&mut build, &plugin_dir.join("src"), 0);
    for dep_dir in ["dep", "deps", "lib"] {
        add_dirs(&mut build, &plugin_dir.join(dep_dir), 0);
    }

    // Symbol renames to avoid cross-plugin collisions
    build.define("pluginInstance", "pluginInstance__cf");
    build.define("init", "init__cf");
    build.define("drwav", "cfdrwav");
    build.define("modeldrwav", "modelcfdrwav");
    build.define("drwavWidget", "cfdrwavWidget");
    build.define("drwav__on_read", "cfdrwav__on_read");
    build.define("modeldrwav__on_read", "modelcfdrwav__on_read");
    build.define("drwav__on_readWidget", "cfdrwav__on_readWidget");
    build.define("drwav__on_seek", "cfdrwav__on_seek");
    build.define("modeldrwav__on_seek", "modelcfdrwav__on_seek");
    build.define("drwav__on_seekWidget", "cfdrwav__on_seekWidget");
    build.define("drwav__read_and_close_f32", "cfdrwav__read_and_close_f32");
    build.define("modeldrwav__read_and_close_f32", "modelcfdrwav__read_and_close_f32");
    build.define("drwav__read_and_close_f32Widget", "cfdrwav__read_and_close_f32Widget");
    build.define("drwav__read_and_close_s16", "cfdrwav__read_and_close_s16");
    build.define("modeldrwav__read_and_close_s16", "modelcfdrwav__read_and_close_s16");
    build.define("drwav__read_and_close_s16Widget", "cfdrwav__read_and_close_s16Widget");
    build.define("drwav__read_and_close_s32", "cfdrwav__read_and_close_s32");
    build.define("modeldrwav__read_and_close_s32", "modelcfdrwav__read_and_close_s32");
    build.define("drwav__read_and_close_s32Widget", "cfdrwav__read_and_close_s32Widget");
    build.define("drwav_alaw_to_f32", "cfdrwav_alaw_to_f32");
    build.define("modeldrwav_alaw_to_f32", "modelcfdrwav_alaw_to_f32");
    build.define("drwav_alaw_to_f32Widget", "cfdrwav_alaw_to_f32Widget");
    build.define("drwav_alaw_to_s16", "cfdrwav_alaw_to_s16");
    build.define("modeldrwav_alaw_to_s16", "modelcfdrwav_alaw_to_s16");
    build.define("drwav_alaw_to_s16Widget", "cfdrwav_alaw_to_s16Widget");
    build.define("drwav_alaw_to_s16", "cfdrwav_alaw_to_s16");
    build.define("modeldrwav_alaw_to_s16", "modelcfdrwav_alaw_to_s16");
    build.define("drwav_alaw_to_s16Widget", "cfdrwav_alaw_to_s16Widget");
    build.define("drwav_alaw_to_s32", "cfdrwav_alaw_to_s32");
    build.define("modeldrwav_alaw_to_s32", "modelcfdrwav_alaw_to_s32");
    build.define("drwav_alaw_to_s32Widget", "cfdrwav_alaw_to_s32Widget");
    build.define("drwav_bytes_to_f32", "cfdrwav_bytes_to_f32");
    build.define("modeldrwav_bytes_to_f32", "modelcfdrwav_bytes_to_f32");
    build.define("drwav_bytes_to_f32Widget", "cfdrwav_bytes_to_f32Widget");
    build.define("drwav_bytes_to_s16", "cfdrwav_bytes_to_s16");
    build.define("modeldrwav_bytes_to_s16", "modelcfdrwav_bytes_to_s16");
    build.define("drwav_bytes_to_s16Widget", "cfdrwav_bytes_to_s16Widget");
    build.define("drwav_bytes_to_s32", "cfdrwav_bytes_to_s32");
    build.define("modeldrwav_bytes_to_s32", "modelcfdrwav_bytes_to_s32");
    build.define("drwav_bytes_to_s32Widget", "cfdrwav_bytes_to_s32Widget");
    build.define("drwav_bytes_to_s64", "cfdrwav_bytes_to_s64");
    build.define("modeldrwav_bytes_to_s64", "modelcfdrwav_bytes_to_s64");
    build.define("drwav_bytes_to_s64Widget", "cfdrwav_bytes_to_s64Widget");
    build.define("drwav_bytes_to_u16", "cfdrwav_bytes_to_u16");
    build.define("modeldrwav_bytes_to_u16", "modelcfdrwav_bytes_to_u16");
    build.define("drwav_bytes_to_u16Widget", "cfdrwav_bytes_to_u16Widget");
    build.define("drwav_bytes_to_u32", "cfdrwav_bytes_to_u32");
    build.define("modeldrwav_bytes_to_u32", "modelcfdrwav_bytes_to_u32");
    build.define("drwav_bytes_to_u32Widget", "cfdrwav_bytes_to_u32Widget");
    build.define("drwav_bytes_to_u64", "cfdrwav_bytes_to_u64");
    build.define("modeldrwav_bytes_to_u64", "modelcfdrwav_bytes_to_u64");
    build.define("drwav_bytes_to_u64Widget", "cfdrwav_bytes_to_u64Widget");
    build.define("drwav_close", "cfdrwav_close");
    build.define("modeldrwav_close", "modelcfdrwav_close");
    build.define("drwav_closeWidget", "cfdrwav_closeWidget");
    build.define("drwav_close", "cfdrwav_close");
    build.define("modeldrwav_close", "modelcfdrwav_close");
    build.define("drwav_closeWidget", "cfdrwav_closeWidget");
    build.define("drwav_container", "cfdrwav_container");
    build.define("modeldrwav_container", "modelcfdrwav_container");
    build.define("drwav_containerWidget", "cfdrwav_containerWidget");
    build.define("drwav_data_chunk_size_riff", "cfdrwav_data_chunk_size_riff");
    build.define("modeldrwav_data_chunk_size_riff", "modelcfdrwav_data_chunk_size_riff");
    build.define("drwav_data_chunk_size_riffWidget", "cfdrwav_data_chunk_size_riffWidget");
    build.define("drwav_data_chunk_size_w64", "cfdrwav_data_chunk_size_w64");
    build.define("modeldrwav_data_chunk_size_w64", "modelcfdrwav_data_chunk_size_w64");
    build.define("drwav_data_chunk_size_w64Widget", "cfdrwav_data_chunk_size_w64Widget");
    build.define("drwav_data_format", "cfdrwav_data_format");
    build.define("modeldrwav_data_format", "modelcfdrwav_data_format");
    build.define("drwav_data_formatWidget", "cfdrwav_data_formatWidget");
    build.define("drwav_f32_to_s16", "cfdrwav_f32_to_s16");
    build.define("modeldrwav_f32_to_s16", "modelcfdrwav_f32_to_s16");
    build.define("drwav_f32_to_s16Widget", "cfdrwav_f32_to_s16Widget");
    build.define("drwav_f32_to_s32", "cfdrwav_f32_to_s32");
    build.define("modeldrwav_f32_to_s32", "modelcfdrwav_f32_to_s32");
    build.define("drwav_f32_to_s32Widget", "cfdrwav_f32_to_s32Widget");
    build.define("drwav_f64_to_f32", "cfdrwav_f64_to_f32");
    build.define("modeldrwav_f64_to_f32", "modelcfdrwav_f64_to_f32");
    build.define("drwav_f64_to_f32Widget", "cfdrwav_f64_to_f32Widget");
    build.define("drwav_f64_to_s16", "cfdrwav_f64_to_s16");
    build.define("modeldrwav_f64_to_s16", "modelcfdrwav_f64_to_s16");
    build.define("drwav_f64_to_s16Widget", "cfdrwav_f64_to_s16Widget");
    build.define("drwav_f64_to_s16", "cfdrwav_f64_to_s16");
    build.define("modeldrwav_f64_to_s16", "modelcfdrwav_f64_to_s16");
    build.define("drwav_f64_to_s16Widget", "cfdrwav_f64_to_s16Widget");
    build.define("drwav_f64_to_s32", "cfdrwav_f64_to_s32");
    build.define("modeldrwav_f64_to_s32", "modelcfdrwav_f64_to_s32");
    build.define("drwav_f64_to_s32Widget", "cfdrwav_f64_to_s32Widget");
    build.define("drwav_fmt_get_format", "cfdrwav_fmt_get_format");
    build.define("modeldrwav_fmt_get_format", "modelcfdrwav_fmt_get_format");
    build.define("drwav_fmt_get_formatWidget", "cfdrwav_fmt_get_formatWidget");
    build.define("drwav_fopen", "cfdrwav_fopen");
    build.define("modeldrwav_fopen", "modelcfdrwav_fopen");
    build.define("drwav_fopenWidget", "cfdrwav_fopenWidget");
    build.define("drwav_fourcc_equal", "cfdrwav_fourcc_equal");
    build.define("modeldrwav_fourcc_equal", "modelcfdrwav_fourcc_equal");
    build.define("drwav_fourcc_equalWidget", "cfdrwav_fourcc_equalWidget");
    build.define("drwav_free", "cfdrwav_free");
    build.define("modeldrwav_free", "modelcfdrwav_free");
    build.define("drwav_freeWidget", "cfdrwav_freeWidget");
    build.define("drwav_get_cursor_in_pcm_frames", "cfdrwav_get_cursor_in_pcm_frames");
    build.define("modeldrwav_get_cursor_in_pcm_frames", "modelcfdrwav_get_cursor_in_pcm_frames");
    build.define("drwav_get_cursor_in_pcm_framesWidget", "cfdrwav_get_cursor_in_pcm_framesWidget");
    build.define("drwav_get_length_in_pcm_frames", "cfdrwav_get_length_in_pcm_frames");
    build.define("modeldrwav_get_length_in_pcm_frames", "modelcfdrwav_get_length_in_pcm_frames");
    build.define("drwav_get_length_in_pcm_framesWidget", "cfdrwav_get_length_in_pcm_framesWidget");
    build.define("drwav_guid_equal", "cfdrwav_guid_equal");
    build.define("modeldrwav_guid_equal", "modelcfdrwav_guid_equal");
    build.define("drwav_guid_equalWidget", "cfdrwav_guid_equalWidget");
    build.define("drwav_init", "cfdrwav_init");
    build.define("modeldrwav_init", "modelcfdrwav_init");
    build.define("drwav_initWidget", "cfdrwav_initWidget");
    build.define("drwav_init_ex", "cfdrwav_init_ex");
    build.define("modeldrwav_init_ex", "modelcfdrwav_init_ex");
    build.define("drwav_init_exWidget", "cfdrwav_init_exWidget");
    build.define("drwav_init_file", "cfdrwav_init_file");
    build.define("modeldrwav_init_file", "modelcfdrwav_init_file");
    build.define("drwav_init_fileWidget", "cfdrwav_init_fileWidget");
    build.define("drwav_init_file_ex", "cfdrwav_init_file_ex");
    build.define("modeldrwav_init_file_ex", "modelcfdrwav_init_file_ex");
    build.define("drwav_init_file_exWidget", "cfdrwav_init_file_exWidget");
    build.define("drwav_init_file_ex_w", "cfdrwav_init_file_ex_w");
    build.define("modeldrwav_init_file_ex_w", "modelcfdrwav_init_file_ex_w");
    build.define("drwav_init_file_ex_wWidget", "cfdrwav_init_file_ex_wWidget");
    build.define("drwav_init_file_w", "cfdrwav_init_file_w");
    build.define("modeldrwav_init_file_w", "modelcfdrwav_init_file_w");
    build.define("drwav_init_file_wWidget", "cfdrwav_init_file_wWidget");
    build.define("drwav_init_file_with_metadata", "cfdrwav_init_file_with_metadata");
    build.define("modeldrwav_init_file_with_metadata", "modelcfdrwav_init_file_with_metadata");
    build.define("drwav_init_file_with_metadataWidget", "cfdrwav_init_file_with_metadataWidget");
    build.define("drwav_init_file_with_metadata_w", "cfdrwav_init_file_with_metadata_w");
    build.define("modeldrwav_init_file_with_metadata_w", "modelcfdrwav_init_file_with_metadata_w");
    build.define("drwav_init_file_with_metadata_wWidget", "cfdrwav_init_file_with_metadata_wWidget");
    build.define("drwav_init_file_write", "cfdrwav_init_file_write");
    build.define("modeldrwav_init_file_write", "modelcfdrwav_init_file_write");
    build.define("drwav_init_file_writeWidget", "cfdrwav_init_file_writeWidget");
    build.define("drwav_init_file_write", "cfdrwav_init_file_write");
    build.define("modeldrwav_init_file_write", "modelcfdrwav_init_file_write");
    build.define("drwav_init_file_writeWidget", "cfdrwav_init_file_writeWidget");
    build.define("drwav_init_file_write__internal", "cfdrwav_init_file_write__internal");
    build.define("modeldrwav_init_file_write__internal", "modelcfdrwav_init_file_write__internal");
    build.define("drwav_init_file_write__internalWidget", "cfdrwav_init_file_write__internalWidget");
    build.define("drwav_init_file_write__internal", "cfdrwav_init_file_write__internal");
    build.define("modeldrwav_init_file_write__internal", "modelcfdrwav_init_file_write__internal");
    build.define("drwav_init_file_write__internalWidget", "cfdrwav_init_file_write__internalWidget");
    build.define("drwav_init_file_write_sequential", "cfdrwav_init_file_write_sequential");
    build.define("modeldrwav_init_file_write_sequential", "modelcfdrwav_init_file_write_sequential");
    build.define("drwav_init_file_write_sequentialWidget", "cfdrwav_init_file_write_sequentialWidget");
    build.define("drwav_init_file_write_sequential", "cfdrwav_init_file_write_sequential");
    build.define("modeldrwav_init_file_write_sequential", "modelcfdrwav_init_file_write_sequential");
    build.define("drwav_init_file_write_sequentialWidget", "cfdrwav_init_file_write_sequentialWidget");
    build.define("drwav_init_file_write_sequential_pcm_frames", "cfdrwav_init_file_write_sequential_pcm_frames");
    build.define("modeldrwav_init_file_write_sequential_pcm_frames", "modelcfdrwav_init_file_write_sequential_pcm_frames");
    build.define("drwav_init_file_write_sequential_pcm_framesWidget", "cfdrwav_init_file_write_sequential_pcm_framesWidget");
    build.define("drwav_init_file_write_sequential_pcm_frames_w", "cfdrwav_init_file_write_sequential_pcm_frames_w");
    build.define("modeldrwav_init_file_write_sequential_pcm_frames_w", "modelcfdrwav_init_file_write_sequential_pcm_frames_w");
    build.define("drwav_init_file_write_sequential_pcm_frames_wWidget", "cfdrwav_init_file_write_sequential_pcm_frames_wWidget");
    build.define("drwav_init_file_write_sequential_w", "cfdrwav_init_file_write_sequential_w");
    build.define("modeldrwav_init_file_write_sequential_w", "modelcfdrwav_init_file_write_sequential_w");
    build.define("drwav_init_file_write_sequential_wWidget", "cfdrwav_init_file_write_sequential_wWidget");
    build.define("drwav_init_file_write_w", "cfdrwav_init_file_write_w");
    build.define("modeldrwav_init_file_write_w", "modelcfdrwav_init_file_write_w");
    build.define("drwav_init_file_write_wWidget", "cfdrwav_init_file_write_wWidget");
    build.define("drwav_init_memory", "cfdrwav_init_memory");
    build.define("modeldrwav_init_memory", "modelcfdrwav_init_memory");
    build.define("drwav_init_memoryWidget", "cfdrwav_init_memoryWidget");
    build.define("drwav_init_memory_ex", "cfdrwav_init_memory_ex");
    build.define("modeldrwav_init_memory_ex", "modelcfdrwav_init_memory_ex");
    build.define("drwav_init_memory_exWidget", "cfdrwav_init_memory_exWidget");
    build.define("drwav_init_memory_with_metadata", "cfdrwav_init_memory_with_metadata");
    build.define("modeldrwav_init_memory_with_metadata", "modelcfdrwav_init_memory_with_metadata");
    build.define("drwav_init_memory_with_metadataWidget", "cfdrwav_init_memory_with_metadataWidget");
    build.define("drwav_init_memory_write", "cfdrwav_init_memory_write");
    build.define("modeldrwav_init_memory_write", "modelcfdrwav_init_memory_write");
    build.define("drwav_init_memory_writeWidget", "cfdrwav_init_memory_writeWidget");
    build.define("drwav_init_memory_write", "cfdrwav_init_memory_write");
    build.define("modeldrwav_init_memory_write", "modelcfdrwav_init_memory_write");
    build.define("drwav_init_memory_writeWidget", "cfdrwav_init_memory_writeWidget");
    build.define("drwav_init_memory_write__internal", "cfdrwav_init_memory_write__internal");
    build.define("modeldrwav_init_memory_write__internal", "modelcfdrwav_init_memory_write__internal");
    build.define("drwav_init_memory_write__internalWidget", "cfdrwav_init_memory_write__internalWidget");
    build.define("drwav_init_memory_write__internal", "cfdrwav_init_memory_write__internal");
    build.define("modeldrwav_init_memory_write__internal", "modelcfdrwav_init_memory_write__internal");
    build.define("drwav_init_memory_write__internalWidget", "cfdrwav_init_memory_write__internalWidget");
    build.define("drwav_init_memory_write_sequential", "cfdrwav_init_memory_write_sequential");
    build.define("modeldrwav_init_memory_write_sequential", "modelcfdrwav_init_memory_write_sequential");
    build.define("drwav_init_memory_write_sequentialWidget", "cfdrwav_init_memory_write_sequentialWidget");
    build.define("drwav_init_memory_write_sequential_pcm_frames", "cfdrwav_init_memory_write_sequential_pcm_frames");
    build.define("modeldrwav_init_memory_write_sequential_pcm_frames", "modelcfdrwav_init_memory_write_sequential_pcm_frames");
    build.define("drwav_init_memory_write_sequential_pcm_framesWidget", "cfdrwav_init_memory_write_sequential_pcm_framesWidget");
    build.define("drwav_init_with_metadata", "cfdrwav_init_with_metadata");
    build.define("modeldrwav_init_with_metadata", "modelcfdrwav_init_with_metadata");
    build.define("drwav_init_with_metadataWidget", "cfdrwav_init_with_metadataWidget");
    build.define("drwav_init_write", "cfdrwav_init_write");
    build.define("modeldrwav_init_write", "modelcfdrwav_init_write");
    build.define("drwav_init_writeWidget", "cfdrwav_init_writeWidget");
    build.define("drwav_init_write", "cfdrwav_init_write");
    build.define("modeldrwav_init_write", "modelcfdrwav_init_write");
    build.define("drwav_init_writeWidget", "cfdrwav_init_writeWidget");
    build.define("drwav_init_write__internal", "cfdrwav_init_write__internal");
    build.define("modeldrwav_init_write__internal", "modelcfdrwav_init_write__internal");
    build.define("drwav_init_write__internalWidget", "cfdrwav_init_write__internalWidget");
    build.define("drwav_init_write_sequential", "cfdrwav_init_write_sequential");
    build.define("modeldrwav_init_write_sequential", "modelcfdrwav_init_write_sequential");
    build.define("drwav_init_write_sequentialWidget", "cfdrwav_init_write_sequentialWidget");
    build.define("drwav_init_write_sequential_pcm_frames", "cfdrwav_init_write_sequential_pcm_frames");
    build.define("modeldrwav_init_write_sequential_pcm_frames", "modelcfdrwav_init_write_sequential_pcm_frames");
    build.define("drwav_init_write_sequential_pcm_framesWidget", "cfdrwav_init_write_sequential_pcm_framesWidget");
    build.define("drwav_init_write_with_metadata", "cfdrwav_init_write_with_metadata");
    build.define("modeldrwav_init_write_with_metadata", "modelcfdrwav_init_write_with_metadata");
    build.define("drwav_init_write_with_metadataWidget", "cfdrwav_init_write_with_metadataWidget");
    build.define("drwav_metadata", "cfdrwav_metadata");
    build.define("modeldrwav_metadata", "modelcfdrwav_metadata");
    build.define("drwav_metadataWidget", "cfdrwav_metadataWidget");
    build.define("drwav__metadata_parser", "cfdrwav__metadata_parser");
    build.define("modeldrwav__metadata_parser", "modelcfdrwav__metadata_parser");
    build.define("drwav__metadata_parserWidget", "cfdrwav__metadata_parserWidget");
    build.define("drwav_mulaw_to_f32", "cfdrwav_mulaw_to_f32");
    build.define("modeldrwav_mulaw_to_f32", "modelcfdrwav_mulaw_to_f32");
    build.define("drwav_mulaw_to_f32Widget", "cfdrwav_mulaw_to_f32Widget");
    build.define("drwav_mulaw_to_s16", "cfdrwav_mulaw_to_s16");
    build.define("modeldrwav_mulaw_to_s16", "modelcfdrwav_mulaw_to_s16");
    build.define("drwav_mulaw_to_s16Widget", "cfdrwav_mulaw_to_s16Widget");
    build.define("drwav_mulaw_to_s16", "cfdrwav_mulaw_to_s16");
    build.define("modeldrwav_mulaw_to_s16", "modelcfdrwav_mulaw_to_s16");
    build.define("drwav_mulaw_to_s16Widget", "cfdrwav_mulaw_to_s16Widget");
    build.define("drwav_mulaw_to_s32", "cfdrwav_mulaw_to_s32");
    build.define("modeldrwav_mulaw_to_s32", "modelcfdrwav_mulaw_to_s32");
    build.define("drwav_mulaw_to_s32Widget", "cfdrwav_mulaw_to_s32Widget");
    build.define("drwav_open", "cfdrwav_open");
    build.define("modeldrwav_open", "modelcfdrwav_open");
    build.define("drwav_openWidget", "cfdrwav_openWidget");
    build.define("drwav_open_and_read_f32", "cfdrwav_open_and_read_f32");
    build.define("modeldrwav_open_and_read_f32", "modelcfdrwav_open_and_read_f32");
    build.define("drwav_open_and_read_f32Widget", "cfdrwav_open_and_read_f32Widget");
    build.define("drwav_open_and_read_file_f32", "cfdrwav_open_and_read_file_f32");
    build.define("modeldrwav_open_and_read_file_f32", "modelcfdrwav_open_and_read_file_f32");
    build.define("drwav_open_and_read_file_f32Widget", "cfdrwav_open_and_read_file_f32Widget");
    build.define("drwav_open_and_read_file_s16", "cfdrwav_open_and_read_file_s16");
    build.define("modeldrwav_open_and_read_file_s16", "modelcfdrwav_open_and_read_file_s16");
    build.define("drwav_open_and_read_file_s16Widget", "cfdrwav_open_and_read_file_s16Widget");
    build.define("drwav_open_and_read_file_s32", "cfdrwav_open_and_read_file_s32");
    build.define("modeldrwav_open_and_read_file_s32", "modelcfdrwav_open_and_read_file_s32");
    build.define("drwav_open_and_read_file_s32Widget", "cfdrwav_open_and_read_file_s32Widget");
    build.define("drwav_open_and_read_memory_f32", "cfdrwav_open_and_read_memory_f32");
    build.define("modeldrwav_open_and_read_memory_f32", "modelcfdrwav_open_and_read_memory_f32");
    build.define("drwav_open_and_read_memory_f32Widget", "cfdrwav_open_and_read_memory_f32Widget");
    build.define("drwav_open_and_read_memory_s16", "cfdrwav_open_and_read_memory_s16");
    build.define("modeldrwav_open_and_read_memory_s16", "modelcfdrwav_open_and_read_memory_s16");
    build.define("drwav_open_and_read_memory_s16Widget", "cfdrwav_open_and_read_memory_s16Widget");
    build.define("drwav_open_and_read_memory_s32", "cfdrwav_open_and_read_memory_s32");
    build.define("modeldrwav_open_and_read_memory_s32", "modelcfdrwav_open_and_read_memory_s32");
    build.define("drwav_open_and_read_memory_s32Widget", "cfdrwav_open_and_read_memory_s32Widget");
    build.define("drwav_open_and_read_pcm_frames_f32", "cfdrwav_open_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_and_read_pcm_frames_f32", "modelcfdrwav_open_and_read_pcm_frames_f32");
    build.define("drwav_open_and_read_pcm_frames_f32Widget", "cfdrwav_open_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_and_read_pcm_frames_s16", "cfdrwav_open_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_and_read_pcm_frames_s16", "modelcfdrwav_open_and_read_pcm_frames_s16");
    build.define("drwav_open_and_read_pcm_frames_s16Widget", "cfdrwav_open_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_and_read_pcm_frames_s32", "cfdrwav_open_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_and_read_pcm_frames_s32", "modelcfdrwav_open_and_read_pcm_frames_s32");
    build.define("drwav_open_and_read_pcm_frames_s32Widget", "cfdrwav_open_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_and_read_s16", "cfdrwav_open_and_read_s16");
    build.define("modeldrwav_open_and_read_s16", "modelcfdrwav_open_and_read_s16");
    build.define("drwav_open_and_read_s16Widget", "cfdrwav_open_and_read_s16Widget");
    build.define("drwav_open_and_read_s32", "cfdrwav_open_and_read_s32");
    build.define("modeldrwav_open_and_read_s32", "modelcfdrwav_open_and_read_s32");
    build.define("drwav_open_and_read_s32Widget", "cfdrwav_open_and_read_s32Widget");
    build.define("drwav_open_ex", "cfdrwav_open_ex");
    build.define("modeldrwav_open_ex", "modelcfdrwav_open_ex");
    build.define("drwav_open_exWidget", "cfdrwav_open_exWidget");
    build.define("drwav_open_file", "cfdrwav_open_file");
    build.define("modeldrwav_open_file", "modelcfdrwav_open_file");
    build.define("drwav_open_fileWidget", "cfdrwav_open_fileWidget");
    build.define("drwav_open_file_and_read_f32", "cfdrwav_open_file_and_read_f32");
    build.define("modeldrwav_open_file_and_read_f32", "modelcfdrwav_open_file_and_read_f32");
    build.define("drwav_open_file_and_read_f32Widget", "cfdrwav_open_file_and_read_f32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_f32", "cfdrwav_open_file_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_file_and_read_pcm_frames_f32", "modelcfdrwav_open_file_and_read_pcm_frames_f32");
    build.define("drwav_open_file_and_read_pcm_frames_f32Widget", "cfdrwav_open_file_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_f32_w", "cfdrwav_open_file_and_read_pcm_frames_f32_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_f32_w", "modelcfdrwav_open_file_and_read_pcm_frames_f32_w");
    build.define("drwav_open_file_and_read_pcm_frames_f32_wWidget", "cfdrwav_open_file_and_read_pcm_frames_f32_wWidget");
    build.define("drwav_open_file_and_read_pcm_frames_s16", "cfdrwav_open_file_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s16", "modelcfdrwav_open_file_and_read_pcm_frames_s16");
    build.define("drwav_open_file_and_read_pcm_frames_s16Widget", "cfdrwav_open_file_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_file_and_read_pcm_frames_s16_w", "cfdrwav_open_file_and_read_pcm_frames_s16_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s16_w", "modelcfdrwav_open_file_and_read_pcm_frames_s16_w");
    build.define("drwav_open_file_and_read_pcm_frames_s16_wWidget", "cfdrwav_open_file_and_read_pcm_frames_s16_wWidget");
    build.define("drwav_open_file_and_read_pcm_frames_s32", "cfdrwav_open_file_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s32", "modelcfdrwav_open_file_and_read_pcm_frames_s32");
    build.define("drwav_open_file_and_read_pcm_frames_s32Widget", "cfdrwav_open_file_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_s32_w", "cfdrwav_open_file_and_read_pcm_frames_s32_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s32_w", "modelcfdrwav_open_file_and_read_pcm_frames_s32_w");
    build.define("drwav_open_file_and_read_pcm_frames_s32_wWidget", "cfdrwav_open_file_and_read_pcm_frames_s32_wWidget");
    build.define("drwav_open_file_and_read_s16", "cfdrwav_open_file_and_read_s16");
    build.define("modeldrwav_open_file_and_read_s16", "modelcfdrwav_open_file_and_read_s16");
    build.define("drwav_open_file_and_read_s16Widget", "cfdrwav_open_file_and_read_s16Widget");
    build.define("drwav_open_file_and_read_s32", "cfdrwav_open_file_and_read_s32");
    build.define("modeldrwav_open_file_and_read_s32", "modelcfdrwav_open_file_and_read_s32");
    build.define("drwav_open_file_and_read_s32Widget", "cfdrwav_open_file_and_read_s32Widget");
    build.define("drwav_open_file_ex", "cfdrwav_open_file_ex");
    build.define("modeldrwav_open_file_ex", "modelcfdrwav_open_file_ex");
    build.define("drwav_open_file_exWidget", "cfdrwav_open_file_exWidget");
    build.define("drwav_open_file_write", "cfdrwav_open_file_write");
    build.define("modeldrwav_open_file_write", "modelcfdrwav_open_file_write");
    build.define("drwav_open_file_writeWidget", "cfdrwav_open_file_writeWidget");
    build.define("drwav_open_file_write", "cfdrwav_open_file_write");
    build.define("modeldrwav_open_file_write", "modelcfdrwav_open_file_write");
    build.define("drwav_open_file_writeWidget", "cfdrwav_open_file_writeWidget");
    build.define("drwav_open_file_write__internal", "cfdrwav_open_file_write__internal");
    build.define("modeldrwav_open_file_write__internal", "modelcfdrwav_open_file_write__internal");
    build.define("drwav_open_file_write__internalWidget", "cfdrwav_open_file_write__internalWidget");
    build.define("drwav_open_file_write__internal", "cfdrwav_open_file_write__internal");
    build.define("modeldrwav_open_file_write__internal", "modelcfdrwav_open_file_write__internal");
    build.define("drwav_open_file_write__internalWidget", "cfdrwav_open_file_write__internalWidget");
    build.define("drwav_open_file_write_sequential", "cfdrwav_open_file_write_sequential");
    build.define("modeldrwav_open_file_write_sequential", "modelcfdrwav_open_file_write_sequential");
    build.define("drwav_open_file_write_sequentialWidget", "cfdrwav_open_file_write_sequentialWidget");
    build.define("drwav_open_file_write_sequential", "cfdrwav_open_file_write_sequential");
    build.define("modeldrwav_open_file_write_sequential", "modelcfdrwav_open_file_write_sequential");
    build.define("drwav_open_file_write_sequentialWidget", "cfdrwav_open_file_write_sequentialWidget");
    build.define("drwav_open_memory", "cfdrwav_open_memory");
    build.define("modeldrwav_open_memory", "modelcfdrwav_open_memory");
    build.define("drwav_open_memoryWidget", "cfdrwav_open_memoryWidget");
    build.define("drwav_open_memory_and_read_f32", "cfdrwav_open_memory_and_read_f32");
    build.define("modeldrwav_open_memory_and_read_f32", "modelcfdrwav_open_memory_and_read_f32");
    build.define("drwav_open_memory_and_read_f32Widget", "cfdrwav_open_memory_and_read_f32Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_f32", "cfdrwav_open_memory_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_f32", "modelcfdrwav_open_memory_and_read_pcm_frames_f32");
    build.define("drwav_open_memory_and_read_pcm_frames_f32Widget", "cfdrwav_open_memory_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_s16", "cfdrwav_open_memory_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_s16", "modelcfdrwav_open_memory_and_read_pcm_frames_s16");
    build.define("drwav_open_memory_and_read_pcm_frames_s16Widget", "cfdrwav_open_memory_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_s32", "cfdrwav_open_memory_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_s32", "modelcfdrwav_open_memory_and_read_pcm_frames_s32");
    build.define("drwav_open_memory_and_read_pcm_frames_s32Widget", "cfdrwav_open_memory_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_memory_and_read_s16", "cfdrwav_open_memory_and_read_s16");
    build.define("modeldrwav_open_memory_and_read_s16", "modelcfdrwav_open_memory_and_read_s16");
    build.define("drwav_open_memory_and_read_s16Widget", "cfdrwav_open_memory_and_read_s16Widget");
    build.define("drwav_open_memory_and_read_s32", "cfdrwav_open_memory_and_read_s32");
    build.define("modeldrwav_open_memory_and_read_s32", "modelcfdrwav_open_memory_and_read_s32");
    build.define("drwav_open_memory_and_read_s32Widget", "cfdrwav_open_memory_and_read_s32Widget");
    build.define("drwav_open_memory_ex", "cfdrwav_open_memory_ex");
    build.define("modeldrwav_open_memory_ex", "modelcfdrwav_open_memory_ex");
    build.define("drwav_open_memory_exWidget", "cfdrwav_open_memory_exWidget");
    build.define("drwav_open_memory_write", "cfdrwav_open_memory_write");
    build.define("modeldrwav_open_memory_write", "modelcfdrwav_open_memory_write");
    build.define("drwav_open_memory_writeWidget", "cfdrwav_open_memory_writeWidget");
    build.define("drwav_open_memory_write", "cfdrwav_open_memory_write");
    build.define("modeldrwav_open_memory_write", "modelcfdrwav_open_memory_write");
    build.define("drwav_open_memory_writeWidget", "cfdrwav_open_memory_writeWidget");
    build.define("drwav_open_memory_write__internal", "cfdrwav_open_memory_write__internal");
    build.define("modeldrwav_open_memory_write__internal", "modelcfdrwav_open_memory_write__internal");
    build.define("drwav_open_memory_write__internalWidget", "cfdrwav_open_memory_write__internalWidget");
    build.define("drwav_open_memory_write__internal", "cfdrwav_open_memory_write__internal");
    build.define("modeldrwav_open_memory_write__internal", "modelcfdrwav_open_memory_write__internal");
    build.define("drwav_open_memory_write__internalWidget", "cfdrwav_open_memory_write__internalWidget");
    build.define("drwav_open_memory_write_sequential", "cfdrwav_open_memory_write_sequential");
    build.define("modeldrwav_open_memory_write_sequential", "modelcfdrwav_open_memory_write_sequential");
    build.define("drwav_open_memory_write_sequentialWidget", "cfdrwav_open_memory_write_sequentialWidget");
    build.define("drwav_open_write", "cfdrwav_open_write");
    build.define("modeldrwav_open_write", "modelcfdrwav_open_write");
    build.define("drwav_open_writeWidget", "cfdrwav_open_writeWidget");
    build.define("drwav_open_write", "cfdrwav_open_write");
    build.define("modeldrwav_open_write", "modelcfdrwav_open_write");
    build.define("drwav_open_writeWidget", "cfdrwav_open_writeWidget");
    build.define("drwav_open_write__internal", "cfdrwav_open_write__internal");
    build.define("modeldrwav_open_write__internal", "modelcfdrwav_open_write__internal");
    build.define("drwav_open_write__internalWidget", "cfdrwav_open_write__internalWidget");
    build.define("drwav_open_write_sequential", "cfdrwav_open_write_sequential");
    build.define("modeldrwav_open_write_sequential", "modelcfdrwav_open_write_sequential");
    build.define("drwav_open_write_sequentialWidget", "cfdrwav_open_write_sequentialWidget");
    build.define("drwav_read", "cfdrwav_read");
    build.define("modeldrwav_read", "modelcfdrwav_read");
    build.define("drwav_readWidget", "cfdrwav_readWidget");
    build.define("drwav_read_f32", "cfdrwav_read_f32");
    build.define("modeldrwav_read_f32", "modelcfdrwav_read_f32");
    build.define("drwav_read_f32Widget", "cfdrwav_read_f32Widget");
    build.define("drwav_read_f32__alaw", "cfdrwav_read_f32__alaw");
    build.define("modeldrwav_read_f32__alaw", "modelcfdrwav_read_f32__alaw");
    build.define("drwav_read_f32__alawWidget", "cfdrwav_read_f32__alawWidget");
    build.define("drwav_read_f32__alaw", "cfdrwav_read_f32__alaw");
    build.define("modeldrwav_read_f32__alaw", "modelcfdrwav_read_f32__alaw");
    build.define("drwav_read_f32__alawWidget", "cfdrwav_read_f32__alawWidget");
    build.define("drwav_read_f32__ieee", "cfdrwav_read_f32__ieee");
    build.define("modeldrwav_read_f32__ieee", "modelcfdrwav_read_f32__ieee");
    build.define("drwav_read_f32__ieeeWidget", "cfdrwav_read_f32__ieeeWidget");
    build.define("drwav_read_f32__ieee", "cfdrwav_read_f32__ieee");
    build.define("modeldrwav_read_f32__ieee", "modelcfdrwav_read_f32__ieee");
    build.define("drwav_read_f32__ieeeWidget", "cfdrwav_read_f32__ieeeWidget");
    build.define("drwav_read_f32__ima", "cfdrwav_read_f32__ima");
    build.define("modeldrwav_read_f32__ima", "modelcfdrwav_read_f32__ima");
    build.define("drwav_read_f32__imaWidget", "cfdrwav_read_f32__imaWidget");
    build.define("drwav_read_f32__ima", "cfdrwav_read_f32__ima");
    build.define("modeldrwav_read_f32__ima", "modelcfdrwav_read_f32__ima");
    build.define("drwav_read_f32__imaWidget", "cfdrwav_read_f32__imaWidget");
    build.define("drwav_read_f32__msadpcm", "cfdrwav_read_f32__msadpcm");
    build.define("modeldrwav_read_f32__msadpcm", "modelcfdrwav_read_f32__msadpcm");
    build.define("drwav_read_f32__msadpcmWidget", "cfdrwav_read_f32__msadpcmWidget");
    build.define("drwav_read_f32__msadpcm", "cfdrwav_read_f32__msadpcm");
    build.define("modeldrwav_read_f32__msadpcm", "modelcfdrwav_read_f32__msadpcm");
    build.define("drwav_read_f32__msadpcmWidget", "cfdrwav_read_f32__msadpcmWidget");
    build.define("drwav_read_f32__mulaw", "cfdrwav_read_f32__mulaw");
    build.define("modeldrwav_read_f32__mulaw", "modelcfdrwav_read_f32__mulaw");
    build.define("drwav_read_f32__mulawWidget", "cfdrwav_read_f32__mulawWidget");
    build.define("drwav_read_f32__mulaw", "cfdrwav_read_f32__mulaw");
    build.define("modeldrwav_read_f32__mulaw", "modelcfdrwav_read_f32__mulaw");
    build.define("drwav_read_f32__mulawWidget", "cfdrwav_read_f32__mulawWidget");
    build.define("drwav_read_f32__pcm", "cfdrwav_read_f32__pcm");
    build.define("modeldrwav_read_f32__pcm", "modelcfdrwav_read_f32__pcm");
    build.define("drwav_read_f32__pcmWidget", "cfdrwav_read_f32__pcmWidget");
    build.define("drwav_read_f32__pcm", "cfdrwav_read_f32__pcm");
    build.define("modeldrwav_read_f32__pcm", "modelcfdrwav_read_f32__pcm");
    build.define("drwav_read_f32__pcmWidget", "cfdrwav_read_f32__pcmWidget");
    build.define("drwav_read_pcm_frames", "cfdrwav_read_pcm_frames");
    build.define("modeldrwav_read_pcm_frames", "modelcfdrwav_read_pcm_frames");
    build.define("drwav_read_pcm_framesWidget", "cfdrwav_read_pcm_framesWidget");
    build.define("drwav_read_pcm_frames_be", "cfdrwav_read_pcm_frames_be");
    build.define("modeldrwav_read_pcm_frames_be", "modelcfdrwav_read_pcm_frames_be");
    build.define("drwav_read_pcm_frames_beWidget", "cfdrwav_read_pcm_frames_beWidget");
    build.define("drwav_read_pcm_frames_f32", "cfdrwav_read_pcm_frames_f32");
    build.define("modeldrwav_read_pcm_frames_f32", "modelcfdrwav_read_pcm_frames_f32");
    build.define("drwav_read_pcm_frames_f32Widget", "cfdrwav_read_pcm_frames_f32Widget");
    build.define("drwav_read_pcm_frames_f32be", "cfdrwav_read_pcm_frames_f32be");
    build.define("modeldrwav_read_pcm_frames_f32be", "modelcfdrwav_read_pcm_frames_f32be");
    build.define("drwav_read_pcm_frames_f32beWidget", "cfdrwav_read_pcm_frames_f32beWidget");
    build.define("drwav_read_pcm_frames_f32le", "cfdrwav_read_pcm_frames_f32le");
    build.define("modeldrwav_read_pcm_frames_f32le", "modelcfdrwav_read_pcm_frames_f32le");
    build.define("drwav_read_pcm_frames_f32leWidget", "cfdrwav_read_pcm_frames_f32leWidget");
    build.define("drwav_read_pcm_frames_le", "cfdrwav_read_pcm_frames_le");
    build.define("modeldrwav_read_pcm_frames_le", "modelcfdrwav_read_pcm_frames_le");
    build.define("drwav_read_pcm_frames_leWidget", "cfdrwav_read_pcm_frames_leWidget");
    build.define("drwav_read_pcm_frames_s16", "cfdrwav_read_pcm_frames_s16");
    build.define("modeldrwav_read_pcm_frames_s16", "modelcfdrwav_read_pcm_frames_s16");
    build.define("drwav_read_pcm_frames_s16Widget", "cfdrwav_read_pcm_frames_s16Widget");
    build.define("drwav_read_pcm_frames_s16be", "cfdrwav_read_pcm_frames_s16be");
    build.define("modeldrwav_read_pcm_frames_s16be", "modelcfdrwav_read_pcm_frames_s16be");
    build.define("drwav_read_pcm_frames_s16beWidget", "cfdrwav_read_pcm_frames_s16beWidget");
    build.define("drwav_read_pcm_frames_s16le", "cfdrwav_read_pcm_frames_s16le");
    build.define("modeldrwav_read_pcm_frames_s16le", "modelcfdrwav_read_pcm_frames_s16le");
    build.define("drwav_read_pcm_frames_s16leWidget", "cfdrwav_read_pcm_frames_s16leWidget");
    build.define("drwav_read_pcm_frames_s32", "cfdrwav_read_pcm_frames_s32");
    build.define("modeldrwav_read_pcm_frames_s32", "modelcfdrwav_read_pcm_frames_s32");
    build.define("drwav_read_pcm_frames_s32Widget", "cfdrwav_read_pcm_frames_s32Widget");
    build.define("drwav_read_pcm_frames_s32be", "cfdrwav_read_pcm_frames_s32be");
    build.define("modeldrwav_read_pcm_frames_s32be", "modelcfdrwav_read_pcm_frames_s32be");
    build.define("drwav_read_pcm_frames_s32beWidget", "cfdrwav_read_pcm_frames_s32beWidget");
    build.define("drwav_read_pcm_frames_s32le", "cfdrwav_read_pcm_frames_s32le");
    build.define("modeldrwav_read_pcm_frames_s32le", "modelcfdrwav_read_pcm_frames_s32le");
    build.define("drwav_read_pcm_frames_s32leWidget", "cfdrwav_read_pcm_frames_s32leWidget");
    build.define("drwav_read_raw", "cfdrwav_read_raw");
    build.define("modeldrwav_read_raw", "modelcfdrwav_read_raw");
    build.define("drwav_read_rawWidget", "cfdrwav_read_rawWidget");
    build.define("drwav_read_s16", "cfdrwav_read_s16");
    build.define("modeldrwav_read_s16", "modelcfdrwav_read_s16");
    build.define("drwav_read_s16Widget", "cfdrwav_read_s16Widget");
    build.define("drwav_read_s16__alaw", "cfdrwav_read_s16__alaw");
    build.define("modeldrwav_read_s16__alaw", "modelcfdrwav_read_s16__alaw");
    build.define("drwav_read_s16__alawWidget", "cfdrwav_read_s16__alawWidget");
    build.define("drwav_read_s16__ieee", "cfdrwav_read_s16__ieee");
    build.define("modeldrwav_read_s16__ieee", "modelcfdrwav_read_s16__ieee");
    build.define("drwav_read_s16__ieeeWidget", "cfdrwav_read_s16__ieeeWidget");
    build.define("drwav_read_s16__ima", "cfdrwav_read_s16__ima");
    build.define("modeldrwav_read_s16__ima", "modelcfdrwav_read_s16__ima");
    build.define("drwav_read_s16__imaWidget", "cfdrwav_read_s16__imaWidget");
    build.define("drwav_read_s16__msadpcm", "cfdrwav_read_s16__msadpcm");
    build.define("modeldrwav_read_s16__msadpcm", "modelcfdrwav_read_s16__msadpcm");
    build.define("drwav_read_s16__msadpcmWidget", "cfdrwav_read_s16__msadpcmWidget");
    build.define("drwav_read_s16__mulaw", "cfdrwav_read_s16__mulaw");
    build.define("modeldrwav_read_s16__mulaw", "modelcfdrwav_read_s16__mulaw");
    build.define("drwav_read_s16__mulawWidget", "cfdrwav_read_s16__mulawWidget");
    build.define("drwav_read_s16__pcm", "cfdrwav_read_s16__pcm");
    build.define("modeldrwav_read_s16__pcm", "modelcfdrwav_read_s16__pcm");
    build.define("drwav_read_s16__pcmWidget", "cfdrwav_read_s16__pcmWidget");
    build.define("drwav_read_s32", "cfdrwav_read_s32");
    build.define("modeldrwav_read_s32", "modelcfdrwav_read_s32");
    build.define("drwav_read_s32Widget", "cfdrwav_read_s32Widget");
    build.define("drwav_read_s32__alaw", "cfdrwav_read_s32__alaw");
    build.define("modeldrwav_read_s32__alaw", "modelcfdrwav_read_s32__alaw");
    build.define("drwav_read_s32__alawWidget", "cfdrwav_read_s32__alawWidget");
    build.define("drwav_read_s32__alaw", "cfdrwav_read_s32__alaw");
    build.define("modeldrwav_read_s32__alaw", "modelcfdrwav_read_s32__alaw");
    build.define("drwav_read_s32__alawWidget", "cfdrwav_read_s32__alawWidget");
    build.define("drwav_read_s32__ieee", "cfdrwav_read_s32__ieee");
    build.define("modeldrwav_read_s32__ieee", "modelcfdrwav_read_s32__ieee");
    build.define("drwav_read_s32__ieeeWidget", "cfdrwav_read_s32__ieeeWidget");
    build.define("drwav_read_s32__ieee", "cfdrwav_read_s32__ieee");
    build.define("modeldrwav_read_s32__ieee", "modelcfdrwav_read_s32__ieee");
    build.define("drwav_read_s32__ieeeWidget", "cfdrwav_read_s32__ieeeWidget");
    build.define("drwav_read_s32__ima", "cfdrwav_read_s32__ima");
    build.define("modeldrwav_read_s32__ima", "modelcfdrwav_read_s32__ima");
    build.define("drwav_read_s32__imaWidget", "cfdrwav_read_s32__imaWidget");
    build.define("drwav_read_s32__ima", "cfdrwav_read_s32__ima");
    build.define("modeldrwav_read_s32__ima", "modelcfdrwav_read_s32__ima");
    build.define("drwav_read_s32__imaWidget", "cfdrwav_read_s32__imaWidget");
    build.define("drwav_read_s32__msadpcm", "cfdrwav_read_s32__msadpcm");
    build.define("modeldrwav_read_s32__msadpcm", "modelcfdrwav_read_s32__msadpcm");
    build.define("drwav_read_s32__msadpcmWidget", "cfdrwav_read_s32__msadpcmWidget");
    build.define("drwav_read_s32__msadpcm", "cfdrwav_read_s32__msadpcm");
    build.define("modeldrwav_read_s32__msadpcm", "modelcfdrwav_read_s32__msadpcm");
    build.define("drwav_read_s32__msadpcmWidget", "cfdrwav_read_s32__msadpcmWidget");
    build.define("drwav_read_s32__mulaw", "cfdrwav_read_s32__mulaw");
    build.define("modeldrwav_read_s32__mulaw", "modelcfdrwav_read_s32__mulaw");
    build.define("drwav_read_s32__mulawWidget", "cfdrwav_read_s32__mulawWidget");
    build.define("drwav_read_s32__mulaw", "cfdrwav_read_s32__mulaw");
    build.define("modeldrwav_read_s32__mulaw", "modelcfdrwav_read_s32__mulaw");
    build.define("drwav_read_s32__mulawWidget", "cfdrwav_read_s32__mulawWidget");
    build.define("drwav_read_s32__pcm", "cfdrwav_read_s32__pcm");
    build.define("modeldrwav_read_s32__pcm", "modelcfdrwav_read_s32__pcm");
    build.define("drwav_read_s32__pcmWidget", "cfdrwav_read_s32__pcmWidget");
    build.define("drwav_read_s32__pcm", "cfdrwav_read_s32__pcm");
    build.define("modeldrwav_read_s32__pcm", "modelcfdrwav_read_s32__pcm");
    build.define("drwav_read_s32__pcmWidget", "cfdrwav_read_s32__pcmWidget");
    build.define("drwav_riff_chunk_size_riff", "cfdrwav_riff_chunk_size_riff");
    build.define("modeldrwav_riff_chunk_size_riff", "modelcfdrwav_riff_chunk_size_riff");
    build.define("drwav_riff_chunk_size_riffWidget", "cfdrwav_riff_chunk_size_riffWidget");
    build.define("drwav_riff_chunk_size_w64", "cfdrwav_riff_chunk_size_w64");
    build.define("modeldrwav_riff_chunk_size_w64", "modelcfdrwav_riff_chunk_size_w64");
    build.define("drwav_riff_chunk_size_w64Widget", "cfdrwav_riff_chunk_size_w64Widget");
    build.define("drwav_s16_to_f32", "cfdrwav_s16_to_f32");
    build.define("modeldrwav_s16_to_f32", "modelcfdrwav_s16_to_f32");
    build.define("drwav_s16_to_f32Widget", "cfdrwav_s16_to_f32Widget");
    build.define("drwav_s16_to_s32", "cfdrwav_s16_to_s32");
    build.define("modeldrwav_s16_to_s32", "modelcfdrwav_s16_to_s32");
    build.define("drwav_s16_to_s32Widget", "cfdrwav_s16_to_s32Widget");
    build.define("drwav_s24_to_f32", "cfdrwav_s24_to_f32");
    build.define("modeldrwav_s24_to_f32", "modelcfdrwav_s24_to_f32");
    build.define("drwav_s24_to_f32Widget", "cfdrwav_s24_to_f32Widget");
    build.define("drwav_s24_to_s16", "cfdrwav_s24_to_s16");
    build.define("modeldrwav_s24_to_s16", "modelcfdrwav_s24_to_s16");
    build.define("drwav_s24_to_s16Widget", "cfdrwav_s24_to_s16Widget");
    build.define("drwav_s24_to_s16", "cfdrwav_s24_to_s16");
    build.define("modeldrwav_s24_to_s16", "modelcfdrwav_s24_to_s16");
    build.define("drwav_s24_to_s16Widget", "cfdrwav_s24_to_s16Widget");
    build.define("drwav_s24_to_s32", "cfdrwav_s24_to_s32");
    build.define("modeldrwav_s24_to_s32", "modelcfdrwav_s24_to_s32");
    build.define("drwav_s24_to_s32Widget", "cfdrwav_s24_to_s32Widget");
    build.define("drwav_s32_to_f32", "cfdrwav_s32_to_f32");
    build.define("modeldrwav_s32_to_f32", "modelcfdrwav_s32_to_f32");
    build.define("drwav_s32_to_f32Widget", "cfdrwav_s32_to_f32Widget");
    build.define("drwav_s32_to_s16", "cfdrwav_s32_to_s16");
    build.define("modeldrwav_s32_to_s16", "modelcfdrwav_s32_to_s16");
    build.define("drwav_s32_to_s16Widget", "cfdrwav_s32_to_s16Widget");
    build.define("drwav_s32_to_s16", "cfdrwav_s32_to_s16");
    build.define("modeldrwav_s32_to_s16", "modelcfdrwav_s32_to_s16");
    build.define("drwav_s32_to_s16Widget", "cfdrwav_s32_to_s16Widget");
    build.define("drwav_seek_to_pcm_frame", "cfdrwav_seek_to_pcm_frame");
    build.define("modeldrwav_seek_to_pcm_frame", "modelcfdrwav_seek_to_pcm_frame");
    build.define("drwav_seek_to_pcm_frameWidget", "cfdrwav_seek_to_pcm_frameWidget");
    build.define("drwav_seek_to_sample", "cfdrwav_seek_to_sample");
    build.define("modeldrwav_seek_to_sample", "modelcfdrwav_seek_to_sample");
    build.define("drwav_seek_to_sampleWidget", "cfdrwav_seek_to_sampleWidget");
    build.define("drwav_seek_to_sample", "cfdrwav_seek_to_sample");
    build.define("modeldrwav_seek_to_sample", "modelcfdrwav_seek_to_sample");
    build.define("drwav_seek_to_sampleWidget", "cfdrwav_seek_to_sampleWidget");
    build.define("drwav_smpl", "cfdrwav_smpl");
    build.define("modeldrwav_smpl", "modelcfdrwav_smpl");
    build.define("drwav_smplWidget", "cfdrwav_smplWidget");
    build.define("drwav_smpl_loop", "cfdrwav_smpl_loop");
    build.define("modeldrwav_smpl_loop", "modelcfdrwav_smpl_loop");
    build.define("drwav_smpl_loopWidget", "cfdrwav_smpl_loopWidget");
    build.define("drwav_take_ownership_of_metadata", "cfdrwav_take_ownership_of_metadata");
    build.define("modeldrwav_take_ownership_of_metadata", "modelcfdrwav_take_ownership_of_metadata");
    build.define("drwav_take_ownership_of_metadataWidget", "cfdrwav_take_ownership_of_metadataWidget");
    build.define("drwav_target_write_size_bytes", "cfdrwav_target_write_size_bytes");
    build.define("modeldrwav_target_write_size_bytes", "modelcfdrwav_target_write_size_bytes");
    build.define("drwav_target_write_size_bytesWidget", "cfdrwav_target_write_size_bytesWidget");
    build.define("drwav_u8_to_f32", "cfdrwav_u8_to_f32");
    build.define("modeldrwav_u8_to_f32", "modelcfdrwav_u8_to_f32");
    build.define("drwav_u8_to_f32Widget", "cfdrwav_u8_to_f32Widget");
    build.define("drwav_u8_to_s16", "cfdrwav_u8_to_s16");
    build.define("modeldrwav_u8_to_s16", "modelcfdrwav_u8_to_s16");
    build.define("drwav_u8_to_s16Widget", "cfdrwav_u8_to_s16Widget");
    build.define("drwav_u8_to_s16", "cfdrwav_u8_to_s16");
    build.define("modeldrwav_u8_to_s16", "modelcfdrwav_u8_to_s16");
    build.define("drwav_u8_to_s16Widget", "cfdrwav_u8_to_s16Widget");
    build.define("drwav_u8_to_s32", "cfdrwav_u8_to_s32");
    build.define("modeldrwav_u8_to_s32", "modelcfdrwav_u8_to_s32");
    build.define("drwav_u8_to_s32Widget", "cfdrwav_u8_to_s32Widget");
    build.define("drwav_uninit", "cfdrwav_uninit");
    build.define("modeldrwav_uninit", "modelcfdrwav_uninit");
    build.define("drwav_uninitWidget", "cfdrwav_uninitWidget");
    build.define("drwav_version", "cfdrwav_version");
    build.define("modeldrwav_version", "modelcfdrwav_version");
    build.define("drwav_versionWidget", "cfdrwav_versionWidget");
    build.define("drwav_version_string", "cfdrwav_version_string");
    build.define("modeldrwav_version_string", "modelcfdrwav_version_string");
    build.define("drwav_version_stringWidget", "cfdrwav_version_stringWidget");
    build.define("drwav_write", "cfdrwav_write");
    build.define("modeldrwav_write", "modelcfdrwav_write");
    build.define("drwav_writeWidget", "cfdrwav_writeWidget");
    build.define("drwav_write", "cfdrwav_write");
    build.define("modeldrwav_write", "modelcfdrwav_write");
    build.define("drwav_writeWidget", "cfdrwav_writeWidget");
    build.define("drwav_write_pcm_frames", "cfdrwav_write_pcm_frames");
    build.define("modeldrwav_write_pcm_frames", "modelcfdrwav_write_pcm_frames");
    build.define("drwav_write_pcm_framesWidget", "cfdrwav_write_pcm_framesWidget");
    build.define("drwav_write_pcm_frames_be", "cfdrwav_write_pcm_frames_be");
    build.define("modeldrwav_write_pcm_frames_be", "modelcfdrwav_write_pcm_frames_be");
    build.define("drwav_write_pcm_frames_beWidget", "cfdrwav_write_pcm_frames_beWidget");
    build.define("drwav_write_pcm_frames_le", "cfdrwav_write_pcm_frames_le");
    build.define("modeldrwav_write_pcm_frames_le", "modelcfdrwav_write_pcm_frames_le");
    build.define("drwav_write_pcm_frames_leWidget", "cfdrwav_write_pcm_frames_leWidget");
    build.define("drwav_write_raw", "cfdrwav_write_raw");
    build.define("modeldrwav_write_raw", "modelcfdrwav_write_raw");
    build.define("drwav_write_rawWidget", "cfdrwav_write_rawWidget");

    // Filter-out list
    let filter_out: Vec<String> = vec![
        "cf/src/plugin.cpp".to_string(),
    ];

    // Source files

    // Glob cf/src/**/*.cpp|cc|c (recursive)
    fn collect_sources(dir: &std::path::Path, filter_out: &[String], plugins_dir: &std::path::Path, build: &mut cc::Build, depth: u32) {
        if depth > 5 || !dir.exists() { return; }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect_sources(&path, filter_out, plugins_dir, build, depth + 1);
                } else if path.extension().map_or(false, |e| e == "cpp" || e == "cc" || e == "c") {
                    let rel = path.strip_prefix(plugins_dir).unwrap_or(&path).to_str().unwrap_or("").to_string();
                    if !filter_out.contains(&rel) {
                        build.file(&path);
                    }
                }
            }
        }
    }
    collect_sources(&plugins_dir.join("cf/src"), &filter_out, &plugins_dir, &mut build, 0);

    build.compile("cardinal_plugin_cf");
}
