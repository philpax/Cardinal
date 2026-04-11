use std::path::PathBuf;

fn main() {
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // cardinal-rs/
        .parent().unwrap()  // Cardinal/
        .to_path_buf();

    let plugins_dir = cardinal_root.join("plugins");
    let plugin_dir = plugins_dir.join("PdArray");

    if !plugin_dir.exists() {
        eprintln!("Plugin PdArray not found (submodule not initialized?), skipping");
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
    build.define("pluginInstance", "pluginInstance__PdArray");
    build.define("init", "init__PdArray");
    build.define("drwav", "PdArraydrwav");
    build.define("modeldrwav", "modelPdArraydrwav");
    build.define("drwavWidget", "PdArraydrwavWidget");
    build.define("drwav__on_read", "PdArraydrwav__on_read");
    build.define("modeldrwav__on_read", "modelPdArraydrwav__on_read");
    build.define("drwav__on_readWidget", "PdArraydrwav__on_readWidget");
    build.define("drwav__on_seek", "PdArraydrwav__on_seek");
    build.define("modeldrwav__on_seek", "modelPdArraydrwav__on_seek");
    build.define("drwav__on_seekWidget", "PdArraydrwav__on_seekWidget");
    build.define("drwav__read_and_close_f32", "PdArraydrwav__read_and_close_f32");
    build.define("modeldrwav__read_and_close_f32", "modelPdArraydrwav__read_and_close_f32");
    build.define("drwav__read_and_close_f32Widget", "PdArraydrwav__read_and_close_f32Widget");
    build.define("drwav__read_and_close_s16", "PdArraydrwav__read_and_close_s16");
    build.define("modeldrwav__read_and_close_s16", "modelPdArraydrwav__read_and_close_s16");
    build.define("drwav__read_and_close_s16Widget", "PdArraydrwav__read_and_close_s16Widget");
    build.define("drwav__read_and_close_s32", "PdArraydrwav__read_and_close_s32");
    build.define("modeldrwav__read_and_close_s32", "modelPdArraydrwav__read_and_close_s32");
    build.define("drwav__read_and_close_s32Widget", "PdArraydrwav__read_and_close_s32Widget");
    build.define("drwav_alaw_to_f32", "PdArraydrwav_alaw_to_f32");
    build.define("modeldrwav_alaw_to_f32", "modelPdArraydrwav_alaw_to_f32");
    build.define("drwav_alaw_to_f32Widget", "PdArraydrwav_alaw_to_f32Widget");
    build.define("drwav_alaw_to_s16", "PdArraydrwav_alaw_to_s16");
    build.define("modeldrwav_alaw_to_s16", "modelPdArraydrwav_alaw_to_s16");
    build.define("drwav_alaw_to_s16Widget", "PdArraydrwav_alaw_to_s16Widget");
    build.define("drwav_alaw_to_s16", "PdArraydrwav_alaw_to_s16");
    build.define("modeldrwav_alaw_to_s16", "modelPdArraydrwav_alaw_to_s16");
    build.define("drwav_alaw_to_s16Widget", "PdArraydrwav_alaw_to_s16Widget");
    build.define("drwav_alaw_to_s32", "PdArraydrwav_alaw_to_s32");
    build.define("modeldrwav_alaw_to_s32", "modelPdArraydrwav_alaw_to_s32");
    build.define("drwav_alaw_to_s32Widget", "PdArraydrwav_alaw_to_s32Widget");
    build.define("drwav_bytes_to_f32", "PdArraydrwav_bytes_to_f32");
    build.define("modeldrwav_bytes_to_f32", "modelPdArraydrwav_bytes_to_f32");
    build.define("drwav_bytes_to_f32Widget", "PdArraydrwav_bytes_to_f32Widget");
    build.define("drwav_bytes_to_s16", "PdArraydrwav_bytes_to_s16");
    build.define("modeldrwav_bytes_to_s16", "modelPdArraydrwav_bytes_to_s16");
    build.define("drwav_bytes_to_s16Widget", "PdArraydrwav_bytes_to_s16Widget");
    build.define("drwav_bytes_to_s32", "PdArraydrwav_bytes_to_s32");
    build.define("modeldrwav_bytes_to_s32", "modelPdArraydrwav_bytes_to_s32");
    build.define("drwav_bytes_to_s32Widget", "PdArraydrwav_bytes_to_s32Widget");
    build.define("drwav_bytes_to_s64", "PdArraydrwav_bytes_to_s64");
    build.define("modeldrwav_bytes_to_s64", "modelPdArraydrwav_bytes_to_s64");
    build.define("drwav_bytes_to_s64Widget", "PdArraydrwav_bytes_to_s64Widget");
    build.define("drwav_bytes_to_u16", "PdArraydrwav_bytes_to_u16");
    build.define("modeldrwav_bytes_to_u16", "modelPdArraydrwav_bytes_to_u16");
    build.define("drwav_bytes_to_u16Widget", "PdArraydrwav_bytes_to_u16Widget");
    build.define("drwav_bytes_to_u32", "PdArraydrwav_bytes_to_u32");
    build.define("modeldrwav_bytes_to_u32", "modelPdArraydrwav_bytes_to_u32");
    build.define("drwav_bytes_to_u32Widget", "PdArraydrwav_bytes_to_u32Widget");
    build.define("drwav_bytes_to_u64", "PdArraydrwav_bytes_to_u64");
    build.define("modeldrwav_bytes_to_u64", "modelPdArraydrwav_bytes_to_u64");
    build.define("drwav_bytes_to_u64Widget", "PdArraydrwav_bytes_to_u64Widget");
    build.define("drwav_close", "PdArraydrwav_close");
    build.define("modeldrwav_close", "modelPdArraydrwav_close");
    build.define("drwav_closeWidget", "PdArraydrwav_closeWidget");
    build.define("drwav_close", "PdArraydrwav_close");
    build.define("modeldrwav_close", "modelPdArraydrwav_close");
    build.define("drwav_closeWidget", "PdArraydrwav_closeWidget");
    build.define("drwav_container", "PdArraydrwav_container");
    build.define("modeldrwav_container", "modelPdArraydrwav_container");
    build.define("drwav_containerWidget", "PdArraydrwav_containerWidget");
    build.define("drwav_data_chunk_size_riff", "PdArraydrwav_data_chunk_size_riff");
    build.define("modeldrwav_data_chunk_size_riff", "modelPdArraydrwav_data_chunk_size_riff");
    build.define("drwav_data_chunk_size_riffWidget", "PdArraydrwav_data_chunk_size_riffWidget");
    build.define("drwav_data_chunk_size_w64", "PdArraydrwav_data_chunk_size_w64");
    build.define("modeldrwav_data_chunk_size_w64", "modelPdArraydrwav_data_chunk_size_w64");
    build.define("drwav_data_chunk_size_w64Widget", "PdArraydrwav_data_chunk_size_w64Widget");
    build.define("drwav_data_format", "PdArraydrwav_data_format");
    build.define("modeldrwav_data_format", "modelPdArraydrwav_data_format");
    build.define("drwav_data_formatWidget", "PdArraydrwav_data_formatWidget");
    build.define("drwav_f32_to_s16", "PdArraydrwav_f32_to_s16");
    build.define("modeldrwav_f32_to_s16", "modelPdArraydrwav_f32_to_s16");
    build.define("drwav_f32_to_s16Widget", "PdArraydrwav_f32_to_s16Widget");
    build.define("drwav_f32_to_s32", "PdArraydrwav_f32_to_s32");
    build.define("modeldrwav_f32_to_s32", "modelPdArraydrwav_f32_to_s32");
    build.define("drwav_f32_to_s32Widget", "PdArraydrwav_f32_to_s32Widget");
    build.define("drwav_f64_to_f32", "PdArraydrwav_f64_to_f32");
    build.define("modeldrwav_f64_to_f32", "modelPdArraydrwav_f64_to_f32");
    build.define("drwav_f64_to_f32Widget", "PdArraydrwav_f64_to_f32Widget");
    build.define("drwav_f64_to_s16", "PdArraydrwav_f64_to_s16");
    build.define("modeldrwav_f64_to_s16", "modelPdArraydrwav_f64_to_s16");
    build.define("drwav_f64_to_s16Widget", "PdArraydrwav_f64_to_s16Widget");
    build.define("drwav_f64_to_s16", "PdArraydrwav_f64_to_s16");
    build.define("modeldrwav_f64_to_s16", "modelPdArraydrwav_f64_to_s16");
    build.define("drwav_f64_to_s16Widget", "PdArraydrwav_f64_to_s16Widget");
    build.define("drwav_f64_to_s32", "PdArraydrwav_f64_to_s32");
    build.define("modeldrwav_f64_to_s32", "modelPdArraydrwav_f64_to_s32");
    build.define("drwav_f64_to_s32Widget", "PdArraydrwav_f64_to_s32Widget");
    build.define("drwav_fmt_get_format", "PdArraydrwav_fmt_get_format");
    build.define("modeldrwav_fmt_get_format", "modelPdArraydrwav_fmt_get_format");
    build.define("drwav_fmt_get_formatWidget", "PdArraydrwav_fmt_get_formatWidget");
    build.define("drwav_fopen", "PdArraydrwav_fopen");
    build.define("modeldrwav_fopen", "modelPdArraydrwav_fopen");
    build.define("drwav_fopenWidget", "PdArraydrwav_fopenWidget");
    build.define("drwav_fourcc_equal", "PdArraydrwav_fourcc_equal");
    build.define("modeldrwav_fourcc_equal", "modelPdArraydrwav_fourcc_equal");
    build.define("drwav_fourcc_equalWidget", "PdArraydrwav_fourcc_equalWidget");
    build.define("drwav_free", "PdArraydrwav_free");
    build.define("modeldrwav_free", "modelPdArraydrwav_free");
    build.define("drwav_freeWidget", "PdArraydrwav_freeWidget");
    build.define("drwav_get_cursor_in_pcm_frames", "PdArraydrwav_get_cursor_in_pcm_frames");
    build.define("modeldrwav_get_cursor_in_pcm_frames", "modelPdArraydrwav_get_cursor_in_pcm_frames");
    build.define("drwav_get_cursor_in_pcm_framesWidget", "PdArraydrwav_get_cursor_in_pcm_framesWidget");
    build.define("drwav_get_length_in_pcm_frames", "PdArraydrwav_get_length_in_pcm_frames");
    build.define("modeldrwav_get_length_in_pcm_frames", "modelPdArraydrwav_get_length_in_pcm_frames");
    build.define("drwav_get_length_in_pcm_framesWidget", "PdArraydrwav_get_length_in_pcm_framesWidget");
    build.define("drwav_guid_equal", "PdArraydrwav_guid_equal");
    build.define("modeldrwav_guid_equal", "modelPdArraydrwav_guid_equal");
    build.define("drwav_guid_equalWidget", "PdArraydrwav_guid_equalWidget");
    build.define("drwav_init", "PdArraydrwav_init");
    build.define("modeldrwav_init", "modelPdArraydrwav_init");
    build.define("drwav_initWidget", "PdArraydrwav_initWidget");
    build.define("drwav_init_ex", "PdArraydrwav_init_ex");
    build.define("modeldrwav_init_ex", "modelPdArraydrwav_init_ex");
    build.define("drwav_init_exWidget", "PdArraydrwav_init_exWidget");
    build.define("drwav_init_file", "PdArraydrwav_init_file");
    build.define("modeldrwav_init_file", "modelPdArraydrwav_init_file");
    build.define("drwav_init_fileWidget", "PdArraydrwav_init_fileWidget");
    build.define("drwav_init_file_ex", "PdArraydrwav_init_file_ex");
    build.define("modeldrwav_init_file_ex", "modelPdArraydrwav_init_file_ex");
    build.define("drwav_init_file_exWidget", "PdArraydrwav_init_file_exWidget");
    build.define("drwav_init_file_ex_w", "PdArraydrwav_init_file_ex_w");
    build.define("modeldrwav_init_file_ex_w", "modelPdArraydrwav_init_file_ex_w");
    build.define("drwav_init_file_ex_wWidget", "PdArraydrwav_init_file_ex_wWidget");
    build.define("drwav_init_file_w", "PdArraydrwav_init_file_w");
    build.define("modeldrwav_init_file_w", "modelPdArraydrwav_init_file_w");
    build.define("drwav_init_file_wWidget", "PdArraydrwav_init_file_wWidget");
    build.define("drwav_init_file_with_metadata", "PdArraydrwav_init_file_with_metadata");
    build.define("modeldrwav_init_file_with_metadata", "modelPdArraydrwav_init_file_with_metadata");
    build.define("drwav_init_file_with_metadataWidget", "PdArraydrwav_init_file_with_metadataWidget");
    build.define("drwav_init_file_with_metadata_w", "PdArraydrwav_init_file_with_metadata_w");
    build.define("modeldrwav_init_file_with_metadata_w", "modelPdArraydrwav_init_file_with_metadata_w");
    build.define("drwav_init_file_with_metadata_wWidget", "PdArraydrwav_init_file_with_metadata_wWidget");
    build.define("drwav_init_file_write", "PdArraydrwav_init_file_write");
    build.define("modeldrwav_init_file_write", "modelPdArraydrwav_init_file_write");
    build.define("drwav_init_file_writeWidget", "PdArraydrwav_init_file_writeWidget");
    build.define("drwav_init_file_write", "PdArraydrwav_init_file_write");
    build.define("modeldrwav_init_file_write", "modelPdArraydrwav_init_file_write");
    build.define("drwav_init_file_writeWidget", "PdArraydrwav_init_file_writeWidget");
    build.define("drwav_init_file_write__internal", "PdArraydrwav_init_file_write__internal");
    build.define("modeldrwav_init_file_write__internal", "modelPdArraydrwav_init_file_write__internal");
    build.define("drwav_init_file_write__internalWidget", "PdArraydrwav_init_file_write__internalWidget");
    build.define("drwav_init_file_write__internal", "PdArraydrwav_init_file_write__internal");
    build.define("modeldrwav_init_file_write__internal", "modelPdArraydrwav_init_file_write__internal");
    build.define("drwav_init_file_write__internalWidget", "PdArraydrwav_init_file_write__internalWidget");
    build.define("drwav_init_file_write_sequential", "PdArraydrwav_init_file_write_sequential");
    build.define("modeldrwav_init_file_write_sequential", "modelPdArraydrwav_init_file_write_sequential");
    build.define("drwav_init_file_write_sequentialWidget", "PdArraydrwav_init_file_write_sequentialWidget");
    build.define("drwav_init_file_write_sequential", "PdArraydrwav_init_file_write_sequential");
    build.define("modeldrwav_init_file_write_sequential", "modelPdArraydrwav_init_file_write_sequential");
    build.define("drwav_init_file_write_sequentialWidget", "PdArraydrwav_init_file_write_sequentialWidget");
    build.define("drwav_init_file_write_sequential_pcm_frames", "PdArraydrwav_init_file_write_sequential_pcm_frames");
    build.define("modeldrwav_init_file_write_sequential_pcm_frames", "modelPdArraydrwav_init_file_write_sequential_pcm_frames");
    build.define("drwav_init_file_write_sequential_pcm_framesWidget", "PdArraydrwav_init_file_write_sequential_pcm_framesWidget");
    build.define("drwav_init_file_write_sequential_pcm_frames_w", "PdArraydrwav_init_file_write_sequential_pcm_frames_w");
    build.define("modeldrwav_init_file_write_sequential_pcm_frames_w", "modelPdArraydrwav_init_file_write_sequential_pcm_frames_w");
    build.define("drwav_init_file_write_sequential_pcm_frames_wWidget", "PdArraydrwav_init_file_write_sequential_pcm_frames_wWidget");
    build.define("drwav_init_file_write_sequential_w", "PdArraydrwav_init_file_write_sequential_w");
    build.define("modeldrwav_init_file_write_sequential_w", "modelPdArraydrwav_init_file_write_sequential_w");
    build.define("drwav_init_file_write_sequential_wWidget", "PdArraydrwav_init_file_write_sequential_wWidget");
    build.define("drwav_init_file_write_w", "PdArraydrwav_init_file_write_w");
    build.define("modeldrwav_init_file_write_w", "modelPdArraydrwav_init_file_write_w");
    build.define("drwav_init_file_write_wWidget", "PdArraydrwav_init_file_write_wWidget");
    build.define("drwav_init_memory", "PdArraydrwav_init_memory");
    build.define("modeldrwav_init_memory", "modelPdArraydrwav_init_memory");
    build.define("drwav_init_memoryWidget", "PdArraydrwav_init_memoryWidget");
    build.define("drwav_init_memory_ex", "PdArraydrwav_init_memory_ex");
    build.define("modeldrwav_init_memory_ex", "modelPdArraydrwav_init_memory_ex");
    build.define("drwav_init_memory_exWidget", "PdArraydrwav_init_memory_exWidget");
    build.define("drwav_init_memory_with_metadata", "PdArraydrwav_init_memory_with_metadata");
    build.define("modeldrwav_init_memory_with_metadata", "modelPdArraydrwav_init_memory_with_metadata");
    build.define("drwav_init_memory_with_metadataWidget", "PdArraydrwav_init_memory_with_metadataWidget");
    build.define("drwav_init_memory_write", "PdArraydrwav_init_memory_write");
    build.define("modeldrwav_init_memory_write", "modelPdArraydrwav_init_memory_write");
    build.define("drwav_init_memory_writeWidget", "PdArraydrwav_init_memory_writeWidget");
    build.define("drwav_init_memory_write", "PdArraydrwav_init_memory_write");
    build.define("modeldrwav_init_memory_write", "modelPdArraydrwav_init_memory_write");
    build.define("drwav_init_memory_writeWidget", "PdArraydrwav_init_memory_writeWidget");
    build.define("drwav_init_memory_write__internal", "PdArraydrwav_init_memory_write__internal");
    build.define("modeldrwav_init_memory_write__internal", "modelPdArraydrwav_init_memory_write__internal");
    build.define("drwav_init_memory_write__internalWidget", "PdArraydrwav_init_memory_write__internalWidget");
    build.define("drwav_init_memory_write__internal", "PdArraydrwav_init_memory_write__internal");
    build.define("modeldrwav_init_memory_write__internal", "modelPdArraydrwav_init_memory_write__internal");
    build.define("drwav_init_memory_write__internalWidget", "PdArraydrwav_init_memory_write__internalWidget");
    build.define("drwav_init_memory_write_sequential", "PdArraydrwav_init_memory_write_sequential");
    build.define("modeldrwav_init_memory_write_sequential", "modelPdArraydrwav_init_memory_write_sequential");
    build.define("drwav_init_memory_write_sequentialWidget", "PdArraydrwav_init_memory_write_sequentialWidget");
    build.define("drwav_init_memory_write_sequential_pcm_frames", "PdArraydrwav_init_memory_write_sequential_pcm_frames");
    build.define("modeldrwav_init_memory_write_sequential_pcm_frames", "modelPdArraydrwav_init_memory_write_sequential_pcm_frames");
    build.define("drwav_init_memory_write_sequential_pcm_framesWidget", "PdArraydrwav_init_memory_write_sequential_pcm_framesWidget");
    build.define("drwav_init_with_metadata", "PdArraydrwav_init_with_metadata");
    build.define("modeldrwav_init_with_metadata", "modelPdArraydrwav_init_with_metadata");
    build.define("drwav_init_with_metadataWidget", "PdArraydrwav_init_with_metadataWidget");
    build.define("drwav_init_write", "PdArraydrwav_init_write");
    build.define("modeldrwav_init_write", "modelPdArraydrwav_init_write");
    build.define("drwav_init_writeWidget", "PdArraydrwav_init_writeWidget");
    build.define("drwav_init_write", "PdArraydrwav_init_write");
    build.define("modeldrwav_init_write", "modelPdArraydrwav_init_write");
    build.define("drwav_init_writeWidget", "PdArraydrwav_init_writeWidget");
    build.define("drwav_init_write__internal", "PdArraydrwav_init_write__internal");
    build.define("modeldrwav_init_write__internal", "modelPdArraydrwav_init_write__internal");
    build.define("drwav_init_write__internalWidget", "PdArraydrwav_init_write__internalWidget");
    build.define("drwav_init_write_sequential", "PdArraydrwav_init_write_sequential");
    build.define("modeldrwav_init_write_sequential", "modelPdArraydrwav_init_write_sequential");
    build.define("drwav_init_write_sequentialWidget", "PdArraydrwav_init_write_sequentialWidget");
    build.define("drwav_init_write_sequential_pcm_frames", "PdArraydrwav_init_write_sequential_pcm_frames");
    build.define("modeldrwav_init_write_sequential_pcm_frames", "modelPdArraydrwav_init_write_sequential_pcm_frames");
    build.define("drwav_init_write_sequential_pcm_framesWidget", "PdArraydrwav_init_write_sequential_pcm_framesWidget");
    build.define("drwav_init_write_with_metadata", "PdArraydrwav_init_write_with_metadata");
    build.define("modeldrwav_init_write_with_metadata", "modelPdArraydrwav_init_write_with_metadata");
    build.define("drwav_init_write_with_metadataWidget", "PdArraydrwav_init_write_with_metadataWidget");
    build.define("drwav_metadata", "PdArraydrwav_metadata");
    build.define("modeldrwav_metadata", "modelPdArraydrwav_metadata");
    build.define("drwav_metadataWidget", "PdArraydrwav_metadataWidget");
    build.define("drwav__metadata_parser", "PdArraydrwav__metadata_parser");
    build.define("modeldrwav__metadata_parser", "modelPdArraydrwav__metadata_parser");
    build.define("drwav__metadata_parserWidget", "PdArraydrwav__metadata_parserWidget");
    build.define("drwav_mulaw_to_f32", "PdArraydrwav_mulaw_to_f32");
    build.define("modeldrwav_mulaw_to_f32", "modelPdArraydrwav_mulaw_to_f32");
    build.define("drwav_mulaw_to_f32Widget", "PdArraydrwav_mulaw_to_f32Widget");
    build.define("drwav_mulaw_to_s16", "PdArraydrwav_mulaw_to_s16");
    build.define("modeldrwav_mulaw_to_s16", "modelPdArraydrwav_mulaw_to_s16");
    build.define("drwav_mulaw_to_s16Widget", "PdArraydrwav_mulaw_to_s16Widget");
    build.define("drwav_mulaw_to_s16", "PdArraydrwav_mulaw_to_s16");
    build.define("modeldrwav_mulaw_to_s16", "modelPdArraydrwav_mulaw_to_s16");
    build.define("drwav_mulaw_to_s16Widget", "PdArraydrwav_mulaw_to_s16Widget");
    build.define("drwav_mulaw_to_s32", "PdArraydrwav_mulaw_to_s32");
    build.define("modeldrwav_mulaw_to_s32", "modelPdArraydrwav_mulaw_to_s32");
    build.define("drwav_mulaw_to_s32Widget", "PdArraydrwav_mulaw_to_s32Widget");
    build.define("drwav_open", "PdArraydrwav_open");
    build.define("modeldrwav_open", "modelPdArraydrwav_open");
    build.define("drwav_openWidget", "PdArraydrwav_openWidget");
    build.define("drwav_open_and_read_f32", "PdArraydrwav_open_and_read_f32");
    build.define("modeldrwav_open_and_read_f32", "modelPdArraydrwav_open_and_read_f32");
    build.define("drwav_open_and_read_f32Widget", "PdArraydrwav_open_and_read_f32Widget");
    build.define("drwav_open_and_read_file_f32", "PdArraydrwav_open_and_read_file_f32");
    build.define("modeldrwav_open_and_read_file_f32", "modelPdArraydrwav_open_and_read_file_f32");
    build.define("drwav_open_and_read_file_f32Widget", "PdArraydrwav_open_and_read_file_f32Widget");
    build.define("drwav_open_and_read_file_s16", "PdArraydrwav_open_and_read_file_s16");
    build.define("modeldrwav_open_and_read_file_s16", "modelPdArraydrwav_open_and_read_file_s16");
    build.define("drwav_open_and_read_file_s16Widget", "PdArraydrwav_open_and_read_file_s16Widget");
    build.define("drwav_open_and_read_file_s32", "PdArraydrwav_open_and_read_file_s32");
    build.define("modeldrwav_open_and_read_file_s32", "modelPdArraydrwav_open_and_read_file_s32");
    build.define("drwav_open_and_read_file_s32Widget", "PdArraydrwav_open_and_read_file_s32Widget");
    build.define("drwav_open_and_read_memory_f32", "PdArraydrwav_open_and_read_memory_f32");
    build.define("modeldrwav_open_and_read_memory_f32", "modelPdArraydrwav_open_and_read_memory_f32");
    build.define("drwav_open_and_read_memory_f32Widget", "PdArraydrwav_open_and_read_memory_f32Widget");
    build.define("drwav_open_and_read_memory_s16", "PdArraydrwav_open_and_read_memory_s16");
    build.define("modeldrwav_open_and_read_memory_s16", "modelPdArraydrwav_open_and_read_memory_s16");
    build.define("drwav_open_and_read_memory_s16Widget", "PdArraydrwav_open_and_read_memory_s16Widget");
    build.define("drwav_open_and_read_memory_s32", "PdArraydrwav_open_and_read_memory_s32");
    build.define("modeldrwav_open_and_read_memory_s32", "modelPdArraydrwav_open_and_read_memory_s32");
    build.define("drwav_open_and_read_memory_s32Widget", "PdArraydrwav_open_and_read_memory_s32Widget");
    build.define("drwav_open_and_read_pcm_frames_f32", "PdArraydrwav_open_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_and_read_pcm_frames_f32", "modelPdArraydrwav_open_and_read_pcm_frames_f32");
    build.define("drwav_open_and_read_pcm_frames_f32Widget", "PdArraydrwav_open_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_and_read_pcm_frames_s16", "PdArraydrwav_open_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_and_read_pcm_frames_s16", "modelPdArraydrwav_open_and_read_pcm_frames_s16");
    build.define("drwav_open_and_read_pcm_frames_s16Widget", "PdArraydrwav_open_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_and_read_pcm_frames_s32", "PdArraydrwav_open_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_and_read_pcm_frames_s32", "modelPdArraydrwav_open_and_read_pcm_frames_s32");
    build.define("drwav_open_and_read_pcm_frames_s32Widget", "PdArraydrwav_open_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_and_read_s16", "PdArraydrwav_open_and_read_s16");
    build.define("modeldrwav_open_and_read_s16", "modelPdArraydrwav_open_and_read_s16");
    build.define("drwav_open_and_read_s16Widget", "PdArraydrwav_open_and_read_s16Widget");
    build.define("drwav_open_and_read_s32", "PdArraydrwav_open_and_read_s32");
    build.define("modeldrwav_open_and_read_s32", "modelPdArraydrwav_open_and_read_s32");
    build.define("drwav_open_and_read_s32Widget", "PdArraydrwav_open_and_read_s32Widget");
    build.define("drwav_open_ex", "PdArraydrwav_open_ex");
    build.define("modeldrwav_open_ex", "modelPdArraydrwav_open_ex");
    build.define("drwav_open_exWidget", "PdArraydrwav_open_exWidget");
    build.define("drwav_open_file", "PdArraydrwav_open_file");
    build.define("modeldrwav_open_file", "modelPdArraydrwav_open_file");
    build.define("drwav_open_fileWidget", "PdArraydrwav_open_fileWidget");
    build.define("drwav_open_file_and_read_f32", "PdArraydrwav_open_file_and_read_f32");
    build.define("modeldrwav_open_file_and_read_f32", "modelPdArraydrwav_open_file_and_read_f32");
    build.define("drwav_open_file_and_read_f32Widget", "PdArraydrwav_open_file_and_read_f32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_f32", "PdArraydrwav_open_file_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_file_and_read_pcm_frames_f32", "modelPdArraydrwav_open_file_and_read_pcm_frames_f32");
    build.define("drwav_open_file_and_read_pcm_frames_f32Widget", "PdArraydrwav_open_file_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_f32_w", "PdArraydrwav_open_file_and_read_pcm_frames_f32_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_f32_w", "modelPdArraydrwav_open_file_and_read_pcm_frames_f32_w");
    build.define("drwav_open_file_and_read_pcm_frames_f32_wWidget", "PdArraydrwav_open_file_and_read_pcm_frames_f32_wWidget");
    build.define("drwav_open_file_and_read_pcm_frames_s16", "PdArraydrwav_open_file_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s16", "modelPdArraydrwav_open_file_and_read_pcm_frames_s16");
    build.define("drwav_open_file_and_read_pcm_frames_s16Widget", "PdArraydrwav_open_file_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_file_and_read_pcm_frames_s16_w", "PdArraydrwav_open_file_and_read_pcm_frames_s16_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s16_w", "modelPdArraydrwav_open_file_and_read_pcm_frames_s16_w");
    build.define("drwav_open_file_and_read_pcm_frames_s16_wWidget", "PdArraydrwav_open_file_and_read_pcm_frames_s16_wWidget");
    build.define("drwav_open_file_and_read_pcm_frames_s32", "PdArraydrwav_open_file_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s32", "modelPdArraydrwav_open_file_and_read_pcm_frames_s32");
    build.define("drwav_open_file_and_read_pcm_frames_s32Widget", "PdArraydrwav_open_file_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_s32_w", "PdArraydrwav_open_file_and_read_pcm_frames_s32_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s32_w", "modelPdArraydrwav_open_file_and_read_pcm_frames_s32_w");
    build.define("drwav_open_file_and_read_pcm_frames_s32_wWidget", "PdArraydrwav_open_file_and_read_pcm_frames_s32_wWidget");
    build.define("drwav_open_file_and_read_s16", "PdArraydrwav_open_file_and_read_s16");
    build.define("modeldrwav_open_file_and_read_s16", "modelPdArraydrwav_open_file_and_read_s16");
    build.define("drwav_open_file_and_read_s16Widget", "PdArraydrwav_open_file_and_read_s16Widget");
    build.define("drwav_open_file_and_read_s32", "PdArraydrwav_open_file_and_read_s32");
    build.define("modeldrwav_open_file_and_read_s32", "modelPdArraydrwav_open_file_and_read_s32");
    build.define("drwav_open_file_and_read_s32Widget", "PdArraydrwav_open_file_and_read_s32Widget");
    build.define("drwav_open_file_ex", "PdArraydrwav_open_file_ex");
    build.define("modeldrwav_open_file_ex", "modelPdArraydrwav_open_file_ex");
    build.define("drwav_open_file_exWidget", "PdArraydrwav_open_file_exWidget");
    build.define("drwav_open_file_write", "PdArraydrwav_open_file_write");
    build.define("modeldrwav_open_file_write", "modelPdArraydrwav_open_file_write");
    build.define("drwav_open_file_writeWidget", "PdArraydrwav_open_file_writeWidget");
    build.define("drwav_open_file_write", "PdArraydrwav_open_file_write");
    build.define("modeldrwav_open_file_write", "modelPdArraydrwav_open_file_write");
    build.define("drwav_open_file_writeWidget", "PdArraydrwav_open_file_writeWidget");
    build.define("drwav_open_file_write__internal", "PdArraydrwav_open_file_write__internal");
    build.define("modeldrwav_open_file_write__internal", "modelPdArraydrwav_open_file_write__internal");
    build.define("drwav_open_file_write__internalWidget", "PdArraydrwav_open_file_write__internalWidget");
    build.define("drwav_open_file_write__internal", "PdArraydrwav_open_file_write__internal");
    build.define("modeldrwav_open_file_write__internal", "modelPdArraydrwav_open_file_write__internal");
    build.define("drwav_open_file_write__internalWidget", "PdArraydrwav_open_file_write__internalWidget");
    build.define("drwav_open_file_write_sequential", "PdArraydrwav_open_file_write_sequential");
    build.define("modeldrwav_open_file_write_sequential", "modelPdArraydrwav_open_file_write_sequential");
    build.define("drwav_open_file_write_sequentialWidget", "PdArraydrwav_open_file_write_sequentialWidget");
    build.define("drwav_open_file_write_sequential", "PdArraydrwav_open_file_write_sequential");
    build.define("modeldrwav_open_file_write_sequential", "modelPdArraydrwav_open_file_write_sequential");
    build.define("drwav_open_file_write_sequentialWidget", "PdArraydrwav_open_file_write_sequentialWidget");
    build.define("drwav_open_memory", "PdArraydrwav_open_memory");
    build.define("modeldrwav_open_memory", "modelPdArraydrwav_open_memory");
    build.define("drwav_open_memoryWidget", "PdArraydrwav_open_memoryWidget");
    build.define("drwav_open_memory_and_read_f32", "PdArraydrwav_open_memory_and_read_f32");
    build.define("modeldrwav_open_memory_and_read_f32", "modelPdArraydrwav_open_memory_and_read_f32");
    build.define("drwav_open_memory_and_read_f32Widget", "PdArraydrwav_open_memory_and_read_f32Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_f32", "PdArraydrwav_open_memory_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_f32", "modelPdArraydrwav_open_memory_and_read_pcm_frames_f32");
    build.define("drwav_open_memory_and_read_pcm_frames_f32Widget", "PdArraydrwav_open_memory_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_s16", "PdArraydrwav_open_memory_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_s16", "modelPdArraydrwav_open_memory_and_read_pcm_frames_s16");
    build.define("drwav_open_memory_and_read_pcm_frames_s16Widget", "PdArraydrwav_open_memory_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_s32", "PdArraydrwav_open_memory_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_s32", "modelPdArraydrwav_open_memory_and_read_pcm_frames_s32");
    build.define("drwav_open_memory_and_read_pcm_frames_s32Widget", "PdArraydrwav_open_memory_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_memory_and_read_s16", "PdArraydrwav_open_memory_and_read_s16");
    build.define("modeldrwav_open_memory_and_read_s16", "modelPdArraydrwav_open_memory_and_read_s16");
    build.define("drwav_open_memory_and_read_s16Widget", "PdArraydrwav_open_memory_and_read_s16Widget");
    build.define("drwav_open_memory_and_read_s32", "PdArraydrwav_open_memory_and_read_s32");
    build.define("modeldrwav_open_memory_and_read_s32", "modelPdArraydrwav_open_memory_and_read_s32");
    build.define("drwav_open_memory_and_read_s32Widget", "PdArraydrwav_open_memory_and_read_s32Widget");
    build.define("drwav_open_memory_ex", "PdArraydrwav_open_memory_ex");
    build.define("modeldrwav_open_memory_ex", "modelPdArraydrwav_open_memory_ex");
    build.define("drwav_open_memory_exWidget", "PdArraydrwav_open_memory_exWidget");
    build.define("drwav_open_memory_write", "PdArraydrwav_open_memory_write");
    build.define("modeldrwav_open_memory_write", "modelPdArraydrwav_open_memory_write");
    build.define("drwav_open_memory_writeWidget", "PdArraydrwav_open_memory_writeWidget");
    build.define("drwav_open_memory_write", "PdArraydrwav_open_memory_write");
    build.define("modeldrwav_open_memory_write", "modelPdArraydrwav_open_memory_write");
    build.define("drwav_open_memory_writeWidget", "PdArraydrwav_open_memory_writeWidget");
    build.define("drwav_open_memory_write__internal", "PdArraydrwav_open_memory_write__internal");
    build.define("modeldrwav_open_memory_write__internal", "modelPdArraydrwav_open_memory_write__internal");
    build.define("drwav_open_memory_write__internalWidget", "PdArraydrwav_open_memory_write__internalWidget");
    build.define("drwav_open_memory_write__internal", "PdArraydrwav_open_memory_write__internal");
    build.define("modeldrwav_open_memory_write__internal", "modelPdArraydrwav_open_memory_write__internal");
    build.define("drwav_open_memory_write__internalWidget", "PdArraydrwav_open_memory_write__internalWidget");
    build.define("drwav_open_memory_write_sequential", "PdArraydrwav_open_memory_write_sequential");
    build.define("modeldrwav_open_memory_write_sequential", "modelPdArraydrwav_open_memory_write_sequential");
    build.define("drwav_open_memory_write_sequentialWidget", "PdArraydrwav_open_memory_write_sequentialWidget");
    build.define("drwav_open_write", "PdArraydrwav_open_write");
    build.define("modeldrwav_open_write", "modelPdArraydrwav_open_write");
    build.define("drwav_open_writeWidget", "PdArraydrwav_open_writeWidget");
    build.define("drwav_open_write", "PdArraydrwav_open_write");
    build.define("modeldrwav_open_write", "modelPdArraydrwav_open_write");
    build.define("drwav_open_writeWidget", "PdArraydrwav_open_writeWidget");
    build.define("drwav_open_write__internal", "PdArraydrwav_open_write__internal");
    build.define("modeldrwav_open_write__internal", "modelPdArraydrwav_open_write__internal");
    build.define("drwav_open_write__internalWidget", "PdArraydrwav_open_write__internalWidget");
    build.define("drwav_open_write_sequential", "PdArraydrwav_open_write_sequential");
    build.define("modeldrwav_open_write_sequential", "modelPdArraydrwav_open_write_sequential");
    build.define("drwav_open_write_sequentialWidget", "PdArraydrwav_open_write_sequentialWidget");
    build.define("drwav_read", "PdArraydrwav_read");
    build.define("modeldrwav_read", "modelPdArraydrwav_read");
    build.define("drwav_readWidget", "PdArraydrwav_readWidget");
    build.define("drwav_read_f32", "PdArraydrwav_read_f32");
    build.define("modeldrwav_read_f32", "modelPdArraydrwav_read_f32");
    build.define("drwav_read_f32Widget", "PdArraydrwav_read_f32Widget");
    build.define("drwav_read_f32__alaw", "PdArraydrwav_read_f32__alaw");
    build.define("modeldrwav_read_f32__alaw", "modelPdArraydrwav_read_f32__alaw");
    build.define("drwav_read_f32__alawWidget", "PdArraydrwav_read_f32__alawWidget");
    build.define("drwav_read_f32__alaw", "PdArraydrwav_read_f32__alaw");
    build.define("modeldrwav_read_f32__alaw", "modelPdArraydrwav_read_f32__alaw");
    build.define("drwav_read_f32__alawWidget", "PdArraydrwav_read_f32__alawWidget");
    build.define("drwav_read_f32__ieee", "PdArraydrwav_read_f32__ieee");
    build.define("modeldrwav_read_f32__ieee", "modelPdArraydrwav_read_f32__ieee");
    build.define("drwav_read_f32__ieeeWidget", "PdArraydrwav_read_f32__ieeeWidget");
    build.define("drwav_read_f32__ieee", "PdArraydrwav_read_f32__ieee");
    build.define("modeldrwav_read_f32__ieee", "modelPdArraydrwav_read_f32__ieee");
    build.define("drwav_read_f32__ieeeWidget", "PdArraydrwav_read_f32__ieeeWidget");
    build.define("drwav_read_f32__ima", "PdArraydrwav_read_f32__ima");
    build.define("modeldrwav_read_f32__ima", "modelPdArraydrwav_read_f32__ima");
    build.define("drwav_read_f32__imaWidget", "PdArraydrwav_read_f32__imaWidget");
    build.define("drwav_read_f32__ima", "PdArraydrwav_read_f32__ima");
    build.define("modeldrwav_read_f32__ima", "modelPdArraydrwav_read_f32__ima");
    build.define("drwav_read_f32__imaWidget", "PdArraydrwav_read_f32__imaWidget");
    build.define("drwav_read_f32__msadpcm", "PdArraydrwav_read_f32__msadpcm");
    build.define("modeldrwav_read_f32__msadpcm", "modelPdArraydrwav_read_f32__msadpcm");
    build.define("drwav_read_f32__msadpcmWidget", "PdArraydrwav_read_f32__msadpcmWidget");
    build.define("drwav_read_f32__msadpcm", "PdArraydrwav_read_f32__msadpcm");
    build.define("modeldrwav_read_f32__msadpcm", "modelPdArraydrwav_read_f32__msadpcm");
    build.define("drwav_read_f32__msadpcmWidget", "PdArraydrwav_read_f32__msadpcmWidget");
    build.define("drwav_read_f32__mulaw", "PdArraydrwav_read_f32__mulaw");
    build.define("modeldrwav_read_f32__mulaw", "modelPdArraydrwav_read_f32__mulaw");
    build.define("drwav_read_f32__mulawWidget", "PdArraydrwav_read_f32__mulawWidget");
    build.define("drwav_read_f32__mulaw", "PdArraydrwav_read_f32__mulaw");
    build.define("modeldrwav_read_f32__mulaw", "modelPdArraydrwav_read_f32__mulaw");
    build.define("drwav_read_f32__mulawWidget", "PdArraydrwav_read_f32__mulawWidget");
    build.define("drwav_read_f32__pcm", "PdArraydrwav_read_f32__pcm");
    build.define("modeldrwav_read_f32__pcm", "modelPdArraydrwav_read_f32__pcm");
    build.define("drwav_read_f32__pcmWidget", "PdArraydrwav_read_f32__pcmWidget");
    build.define("drwav_read_f32__pcm", "PdArraydrwav_read_f32__pcm");
    build.define("modeldrwav_read_f32__pcm", "modelPdArraydrwav_read_f32__pcm");
    build.define("drwav_read_f32__pcmWidget", "PdArraydrwav_read_f32__pcmWidget");
    build.define("drwav_read_pcm_frames", "PdArraydrwav_read_pcm_frames");
    build.define("modeldrwav_read_pcm_frames", "modelPdArraydrwav_read_pcm_frames");
    build.define("drwav_read_pcm_framesWidget", "PdArraydrwav_read_pcm_framesWidget");
    build.define("drwav_read_pcm_frames_be", "PdArraydrwav_read_pcm_frames_be");
    build.define("modeldrwav_read_pcm_frames_be", "modelPdArraydrwav_read_pcm_frames_be");
    build.define("drwav_read_pcm_frames_beWidget", "PdArraydrwav_read_pcm_frames_beWidget");
    build.define("drwav_read_pcm_frames_f32", "PdArraydrwav_read_pcm_frames_f32");
    build.define("modeldrwav_read_pcm_frames_f32", "modelPdArraydrwav_read_pcm_frames_f32");
    build.define("drwav_read_pcm_frames_f32Widget", "PdArraydrwav_read_pcm_frames_f32Widget");
    build.define("drwav_read_pcm_frames_f32be", "PdArraydrwav_read_pcm_frames_f32be");
    build.define("modeldrwav_read_pcm_frames_f32be", "modelPdArraydrwav_read_pcm_frames_f32be");
    build.define("drwav_read_pcm_frames_f32beWidget", "PdArraydrwav_read_pcm_frames_f32beWidget");
    build.define("drwav_read_pcm_frames_f32le", "PdArraydrwav_read_pcm_frames_f32le");
    build.define("modeldrwav_read_pcm_frames_f32le", "modelPdArraydrwav_read_pcm_frames_f32le");
    build.define("drwav_read_pcm_frames_f32leWidget", "PdArraydrwav_read_pcm_frames_f32leWidget");
    build.define("drwav_read_pcm_frames_le", "PdArraydrwav_read_pcm_frames_le");
    build.define("modeldrwav_read_pcm_frames_le", "modelPdArraydrwav_read_pcm_frames_le");
    build.define("drwav_read_pcm_frames_leWidget", "PdArraydrwav_read_pcm_frames_leWidget");
    build.define("drwav_read_pcm_frames_s16", "PdArraydrwav_read_pcm_frames_s16");
    build.define("modeldrwav_read_pcm_frames_s16", "modelPdArraydrwav_read_pcm_frames_s16");
    build.define("drwav_read_pcm_frames_s16Widget", "PdArraydrwav_read_pcm_frames_s16Widget");
    build.define("drwav_read_pcm_frames_s16be", "PdArraydrwav_read_pcm_frames_s16be");
    build.define("modeldrwav_read_pcm_frames_s16be", "modelPdArraydrwav_read_pcm_frames_s16be");
    build.define("drwav_read_pcm_frames_s16beWidget", "PdArraydrwav_read_pcm_frames_s16beWidget");
    build.define("drwav_read_pcm_frames_s16le", "PdArraydrwav_read_pcm_frames_s16le");
    build.define("modeldrwav_read_pcm_frames_s16le", "modelPdArraydrwav_read_pcm_frames_s16le");
    build.define("drwav_read_pcm_frames_s16leWidget", "PdArraydrwav_read_pcm_frames_s16leWidget");
    build.define("drwav_read_pcm_frames_s32", "PdArraydrwav_read_pcm_frames_s32");
    build.define("modeldrwav_read_pcm_frames_s32", "modelPdArraydrwav_read_pcm_frames_s32");
    build.define("drwav_read_pcm_frames_s32Widget", "PdArraydrwav_read_pcm_frames_s32Widget");
    build.define("drwav_read_pcm_frames_s32be", "PdArraydrwav_read_pcm_frames_s32be");
    build.define("modeldrwav_read_pcm_frames_s32be", "modelPdArraydrwav_read_pcm_frames_s32be");
    build.define("drwav_read_pcm_frames_s32beWidget", "PdArraydrwav_read_pcm_frames_s32beWidget");
    build.define("drwav_read_pcm_frames_s32le", "PdArraydrwav_read_pcm_frames_s32le");
    build.define("modeldrwav_read_pcm_frames_s32le", "modelPdArraydrwav_read_pcm_frames_s32le");
    build.define("drwav_read_pcm_frames_s32leWidget", "PdArraydrwav_read_pcm_frames_s32leWidget");
    build.define("drwav_read_raw", "PdArraydrwav_read_raw");
    build.define("modeldrwav_read_raw", "modelPdArraydrwav_read_raw");
    build.define("drwav_read_rawWidget", "PdArraydrwav_read_rawWidget");
    build.define("drwav_read_s16", "PdArraydrwav_read_s16");
    build.define("modeldrwav_read_s16", "modelPdArraydrwav_read_s16");
    build.define("drwav_read_s16Widget", "PdArraydrwav_read_s16Widget");
    build.define("drwav_read_s16__alaw", "PdArraydrwav_read_s16__alaw");
    build.define("modeldrwav_read_s16__alaw", "modelPdArraydrwav_read_s16__alaw");
    build.define("drwav_read_s16__alawWidget", "PdArraydrwav_read_s16__alawWidget");
    build.define("drwav_read_s16__ieee", "PdArraydrwav_read_s16__ieee");
    build.define("modeldrwav_read_s16__ieee", "modelPdArraydrwav_read_s16__ieee");
    build.define("drwav_read_s16__ieeeWidget", "PdArraydrwav_read_s16__ieeeWidget");
    build.define("drwav_read_s16__ima", "PdArraydrwav_read_s16__ima");
    build.define("modeldrwav_read_s16__ima", "modelPdArraydrwav_read_s16__ima");
    build.define("drwav_read_s16__imaWidget", "PdArraydrwav_read_s16__imaWidget");
    build.define("drwav_read_s16__msadpcm", "PdArraydrwav_read_s16__msadpcm");
    build.define("modeldrwav_read_s16__msadpcm", "modelPdArraydrwav_read_s16__msadpcm");
    build.define("drwav_read_s16__msadpcmWidget", "PdArraydrwav_read_s16__msadpcmWidget");
    build.define("drwav_read_s16__mulaw", "PdArraydrwav_read_s16__mulaw");
    build.define("modeldrwav_read_s16__mulaw", "modelPdArraydrwav_read_s16__mulaw");
    build.define("drwav_read_s16__mulawWidget", "PdArraydrwav_read_s16__mulawWidget");
    build.define("drwav_read_s16__pcm", "PdArraydrwav_read_s16__pcm");
    build.define("modeldrwav_read_s16__pcm", "modelPdArraydrwav_read_s16__pcm");
    build.define("drwav_read_s16__pcmWidget", "PdArraydrwav_read_s16__pcmWidget");
    build.define("drwav_read_s32", "PdArraydrwav_read_s32");
    build.define("modeldrwav_read_s32", "modelPdArraydrwav_read_s32");
    build.define("drwav_read_s32Widget", "PdArraydrwav_read_s32Widget");
    build.define("drwav_read_s32__alaw", "PdArraydrwav_read_s32__alaw");
    build.define("modeldrwav_read_s32__alaw", "modelPdArraydrwav_read_s32__alaw");
    build.define("drwav_read_s32__alawWidget", "PdArraydrwav_read_s32__alawWidget");
    build.define("drwav_read_s32__alaw", "PdArraydrwav_read_s32__alaw");
    build.define("modeldrwav_read_s32__alaw", "modelPdArraydrwav_read_s32__alaw");
    build.define("drwav_read_s32__alawWidget", "PdArraydrwav_read_s32__alawWidget");
    build.define("drwav_read_s32__ieee", "PdArraydrwav_read_s32__ieee");
    build.define("modeldrwav_read_s32__ieee", "modelPdArraydrwav_read_s32__ieee");
    build.define("drwav_read_s32__ieeeWidget", "PdArraydrwav_read_s32__ieeeWidget");
    build.define("drwav_read_s32__ieee", "PdArraydrwav_read_s32__ieee");
    build.define("modeldrwav_read_s32__ieee", "modelPdArraydrwav_read_s32__ieee");
    build.define("drwav_read_s32__ieeeWidget", "PdArraydrwav_read_s32__ieeeWidget");
    build.define("drwav_read_s32__ima", "PdArraydrwav_read_s32__ima");
    build.define("modeldrwav_read_s32__ima", "modelPdArraydrwav_read_s32__ima");
    build.define("drwav_read_s32__imaWidget", "PdArraydrwav_read_s32__imaWidget");
    build.define("drwav_read_s32__ima", "PdArraydrwav_read_s32__ima");
    build.define("modeldrwav_read_s32__ima", "modelPdArraydrwav_read_s32__ima");
    build.define("drwav_read_s32__imaWidget", "PdArraydrwav_read_s32__imaWidget");
    build.define("drwav_read_s32__msadpcm", "PdArraydrwav_read_s32__msadpcm");
    build.define("modeldrwav_read_s32__msadpcm", "modelPdArraydrwav_read_s32__msadpcm");
    build.define("drwav_read_s32__msadpcmWidget", "PdArraydrwav_read_s32__msadpcmWidget");
    build.define("drwav_read_s32__msadpcm", "PdArraydrwav_read_s32__msadpcm");
    build.define("modeldrwav_read_s32__msadpcm", "modelPdArraydrwav_read_s32__msadpcm");
    build.define("drwav_read_s32__msadpcmWidget", "PdArraydrwav_read_s32__msadpcmWidget");
    build.define("drwav_read_s32__mulaw", "PdArraydrwav_read_s32__mulaw");
    build.define("modeldrwav_read_s32__mulaw", "modelPdArraydrwav_read_s32__mulaw");
    build.define("drwav_read_s32__mulawWidget", "PdArraydrwav_read_s32__mulawWidget");
    build.define("drwav_read_s32__mulaw", "PdArraydrwav_read_s32__mulaw");
    build.define("modeldrwav_read_s32__mulaw", "modelPdArraydrwav_read_s32__mulaw");
    build.define("drwav_read_s32__mulawWidget", "PdArraydrwav_read_s32__mulawWidget");
    build.define("drwav_read_s32__pcm", "PdArraydrwav_read_s32__pcm");
    build.define("modeldrwav_read_s32__pcm", "modelPdArraydrwav_read_s32__pcm");
    build.define("drwav_read_s32__pcmWidget", "PdArraydrwav_read_s32__pcmWidget");
    build.define("drwav_read_s32__pcm", "PdArraydrwav_read_s32__pcm");
    build.define("modeldrwav_read_s32__pcm", "modelPdArraydrwav_read_s32__pcm");
    build.define("drwav_read_s32__pcmWidget", "PdArraydrwav_read_s32__pcmWidget");
    build.define("drwav_riff_chunk_size_riff", "PdArraydrwav_riff_chunk_size_riff");
    build.define("modeldrwav_riff_chunk_size_riff", "modelPdArraydrwav_riff_chunk_size_riff");
    build.define("drwav_riff_chunk_size_riffWidget", "PdArraydrwav_riff_chunk_size_riffWidget");
    build.define("drwav_riff_chunk_size_w64", "PdArraydrwav_riff_chunk_size_w64");
    build.define("modeldrwav_riff_chunk_size_w64", "modelPdArraydrwav_riff_chunk_size_w64");
    build.define("drwav_riff_chunk_size_w64Widget", "PdArraydrwav_riff_chunk_size_w64Widget");
    build.define("drwav_s16_to_f32", "PdArraydrwav_s16_to_f32");
    build.define("modeldrwav_s16_to_f32", "modelPdArraydrwav_s16_to_f32");
    build.define("drwav_s16_to_f32Widget", "PdArraydrwav_s16_to_f32Widget");
    build.define("drwav_s16_to_s32", "PdArraydrwav_s16_to_s32");
    build.define("modeldrwav_s16_to_s32", "modelPdArraydrwav_s16_to_s32");
    build.define("drwav_s16_to_s32Widget", "PdArraydrwav_s16_to_s32Widget");
    build.define("drwav_s24_to_f32", "PdArraydrwav_s24_to_f32");
    build.define("modeldrwav_s24_to_f32", "modelPdArraydrwav_s24_to_f32");
    build.define("drwav_s24_to_f32Widget", "PdArraydrwav_s24_to_f32Widget");
    build.define("drwav_s24_to_s16", "PdArraydrwav_s24_to_s16");
    build.define("modeldrwav_s24_to_s16", "modelPdArraydrwav_s24_to_s16");
    build.define("drwav_s24_to_s16Widget", "PdArraydrwav_s24_to_s16Widget");
    build.define("drwav_s24_to_s16", "PdArraydrwav_s24_to_s16");
    build.define("modeldrwav_s24_to_s16", "modelPdArraydrwav_s24_to_s16");
    build.define("drwav_s24_to_s16Widget", "PdArraydrwav_s24_to_s16Widget");
    build.define("drwav_s24_to_s32", "PdArraydrwav_s24_to_s32");
    build.define("modeldrwav_s24_to_s32", "modelPdArraydrwav_s24_to_s32");
    build.define("drwav_s24_to_s32Widget", "PdArraydrwav_s24_to_s32Widget");
    build.define("drwav_s32_to_f32", "PdArraydrwav_s32_to_f32");
    build.define("modeldrwav_s32_to_f32", "modelPdArraydrwav_s32_to_f32");
    build.define("drwav_s32_to_f32Widget", "PdArraydrwav_s32_to_f32Widget");
    build.define("drwav_s32_to_s16", "PdArraydrwav_s32_to_s16");
    build.define("modeldrwav_s32_to_s16", "modelPdArraydrwav_s32_to_s16");
    build.define("drwav_s32_to_s16Widget", "PdArraydrwav_s32_to_s16Widget");
    build.define("drwav_s32_to_s16", "PdArraydrwav_s32_to_s16");
    build.define("modeldrwav_s32_to_s16", "modelPdArraydrwav_s32_to_s16");
    build.define("drwav_s32_to_s16Widget", "PdArraydrwav_s32_to_s16Widget");
    build.define("drwav_seek_to_pcm_frame", "PdArraydrwav_seek_to_pcm_frame");
    build.define("modeldrwav_seek_to_pcm_frame", "modelPdArraydrwav_seek_to_pcm_frame");
    build.define("drwav_seek_to_pcm_frameWidget", "PdArraydrwav_seek_to_pcm_frameWidget");
    build.define("drwav_seek_to_sample", "PdArraydrwav_seek_to_sample");
    build.define("modeldrwav_seek_to_sample", "modelPdArraydrwav_seek_to_sample");
    build.define("drwav_seek_to_sampleWidget", "PdArraydrwav_seek_to_sampleWidget");
    build.define("drwav_seek_to_sample", "PdArraydrwav_seek_to_sample");
    build.define("modeldrwav_seek_to_sample", "modelPdArraydrwav_seek_to_sample");
    build.define("drwav_seek_to_sampleWidget", "PdArraydrwav_seek_to_sampleWidget");
    build.define("drwav_smpl", "PdArraydrwav_smpl");
    build.define("modeldrwav_smpl", "modelPdArraydrwav_smpl");
    build.define("drwav_smplWidget", "PdArraydrwav_smplWidget");
    build.define("drwav_smpl_loop", "PdArraydrwav_smpl_loop");
    build.define("modeldrwav_smpl_loop", "modelPdArraydrwav_smpl_loop");
    build.define("drwav_smpl_loopWidget", "PdArraydrwav_smpl_loopWidget");
    build.define("drwav_take_ownership_of_metadata", "PdArraydrwav_take_ownership_of_metadata");
    build.define("modeldrwav_take_ownership_of_metadata", "modelPdArraydrwav_take_ownership_of_metadata");
    build.define("drwav_take_ownership_of_metadataWidget", "PdArraydrwav_take_ownership_of_metadataWidget");
    build.define("drwav_target_write_size_bytes", "PdArraydrwav_target_write_size_bytes");
    build.define("modeldrwav_target_write_size_bytes", "modelPdArraydrwav_target_write_size_bytes");
    build.define("drwav_target_write_size_bytesWidget", "PdArraydrwav_target_write_size_bytesWidget");
    build.define("drwav_u8_to_f32", "PdArraydrwav_u8_to_f32");
    build.define("modeldrwav_u8_to_f32", "modelPdArraydrwav_u8_to_f32");
    build.define("drwav_u8_to_f32Widget", "PdArraydrwav_u8_to_f32Widget");
    build.define("drwav_u8_to_s16", "PdArraydrwav_u8_to_s16");
    build.define("modeldrwav_u8_to_s16", "modelPdArraydrwav_u8_to_s16");
    build.define("drwav_u8_to_s16Widget", "PdArraydrwav_u8_to_s16Widget");
    build.define("drwav_u8_to_s16", "PdArraydrwav_u8_to_s16");
    build.define("modeldrwav_u8_to_s16", "modelPdArraydrwav_u8_to_s16");
    build.define("drwav_u8_to_s16Widget", "PdArraydrwav_u8_to_s16Widget");
    build.define("drwav_u8_to_s32", "PdArraydrwav_u8_to_s32");
    build.define("modeldrwav_u8_to_s32", "modelPdArraydrwav_u8_to_s32");
    build.define("drwav_u8_to_s32Widget", "PdArraydrwav_u8_to_s32Widget");
    build.define("drwav_uninit", "PdArraydrwav_uninit");
    build.define("modeldrwav_uninit", "modelPdArraydrwav_uninit");
    build.define("drwav_uninitWidget", "PdArraydrwav_uninitWidget");
    build.define("drwav_version", "PdArraydrwav_version");
    build.define("modeldrwav_version", "modelPdArraydrwav_version");
    build.define("drwav_versionWidget", "PdArraydrwav_versionWidget");
    build.define("drwav_version_string", "PdArraydrwav_version_string");
    build.define("modeldrwav_version_string", "modelPdArraydrwav_version_string");
    build.define("drwav_version_stringWidget", "PdArraydrwav_version_stringWidget");
    build.define("drwav_write", "PdArraydrwav_write");
    build.define("modeldrwav_write", "modelPdArraydrwav_write");
    build.define("drwav_writeWidget", "PdArraydrwav_writeWidget");
    build.define("drwav_write", "PdArraydrwav_write");
    build.define("modeldrwav_write", "modelPdArraydrwav_write");
    build.define("drwav_writeWidget", "PdArraydrwav_writeWidget");
    build.define("drwav_write_pcm_frames", "PdArraydrwav_write_pcm_frames");
    build.define("modeldrwav_write_pcm_frames", "modelPdArraydrwav_write_pcm_frames");
    build.define("drwav_write_pcm_framesWidget", "PdArraydrwav_write_pcm_framesWidget");
    build.define("drwav_write_pcm_frames_be", "PdArraydrwav_write_pcm_frames_be");
    build.define("modeldrwav_write_pcm_frames_be", "modelPdArraydrwav_write_pcm_frames_be");
    build.define("drwav_write_pcm_frames_beWidget", "PdArraydrwav_write_pcm_frames_beWidget");
    build.define("drwav_write_pcm_frames_le", "PdArraydrwav_write_pcm_frames_le");
    build.define("modeldrwav_write_pcm_frames_le", "modelPdArraydrwav_write_pcm_frames_le");
    build.define("drwav_write_pcm_frames_leWidget", "PdArraydrwav_write_pcm_frames_leWidget");
    build.define("drwav_write_raw", "PdArraydrwav_write_raw");
    build.define("modeldrwav_write_raw", "modelPdArraydrwav_write_raw");
    build.define("drwav_write_rawWidget", "PdArraydrwav_write_rawWidget");
    build.define("CustomTrimpot", "PdArrayCustomTrimpot");
    build.define("modelCustomTrimpot", "modelPdArrayCustomTrimpot");
    build.define("CustomTrimpotWidget", "PdArrayCustomTrimpotWidget");
    build.define("MsDisplayWidget", "PdArrayMsDisplayWidget");
    build.define("modelMsDisplayWidget", "modelPdArrayMsDisplayWidget");
    build.define("MsDisplayWidgetWidget", "PdArrayMsDisplayWidgetWidget");
    build.define("TextBox", "PdArrayTextBox");
    build.define("modelTextBox", "modelPdArrayTextBox");
    build.define("TextBoxWidget", "PdArrayTextBoxWidget");

    // Filter-out list
    let _filter_out: Vec<String> = vec![

    ];

    // Source files

    // Recursively collect source files, skipping test/template dirs
    #[allow(dead_code)]
    fn collect_sources(dir: &std::path::Path, filter_out: &[String], plugins_dir: &std::path::Path, build: &mut cc::Build, depth: u32) {
        if depth > 5 || !dir.exists() { return; }
        // Skip directories that contain test/template/example files
        if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            if lower == "test" || lower == "tests" || lower == "template"
                || lower == "templates" || lower == "examples" || lower == "doc"
                || lower == "docs" || lower == "benchmark" || lower == "benchmarks" {
                return;
            }
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect_sources(&path, filter_out, plugins_dir, build, depth + 1);
                } else if path.extension().map_or(false, |e| e == "cpp" || e == "cc" || e == "c") {
                    // Skip test files by name
                    let fname = path.file_name().unwrap_or_default().to_str().unwrap_or("");
                    if fname.contains("_test") || fname.contains("test_") || fname == "main.cpp" || fname == "main.c" {
                        continue;
                    }
                    let rel = path.strip_prefix(plugins_dir).unwrap_or(&path).to_str().unwrap_or("").to_string();
                    if !filter_out.contains(&rel) {
                        build.file(&path);
                    }
                }
            }
        }
    }
    collect_sources(&plugins_dir.join("PdArray/src"), &_filter_out, &plugins_dir, &mut build, 0);

    build.compile("cardinal_plugin_pdarray");
}
