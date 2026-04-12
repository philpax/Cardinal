use std::path::PathBuf;

fn main() {
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // cardinal-rs/
        .parent().unwrap()  // Cardinal/
        .to_path_buf();

    let plugins_dir = cardinal_root.join("plugins");
    let plugin_dir = plugins_dir.join("Fundamental");

    if !plugin_dir.exists() {
        eprintln!("Plugin Fundamental not found (submodule not initialized?), skipping");
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
    build.define("pluginInstance", "pluginInstance__Fundamental");
    build.define("drwav", "Fundamentaldrwav");
    build.define("modeldrwav", "modelFundamentaldrwav");
    build.define("drwavWidget", "FundamentaldrwavWidget");
    build.define("drwav__on_read", "Fundamentaldrwav__on_read");
    build.define("modeldrwav__on_read", "modelFundamentaldrwav__on_read");
    build.define("drwav__on_readWidget", "Fundamentaldrwav__on_readWidget");
    build.define("drwav__on_seek", "Fundamentaldrwav__on_seek");
    build.define("modeldrwav__on_seek", "modelFundamentaldrwav__on_seek");
    build.define("drwav__on_seekWidget", "Fundamentaldrwav__on_seekWidget");
    build.define("drwav__read_and_close_f32", "Fundamentaldrwav__read_and_close_f32");
    build.define("modeldrwav__read_and_close_f32", "modelFundamentaldrwav__read_and_close_f32");
    build.define("drwav__read_and_close_f32Widget", "Fundamentaldrwav__read_and_close_f32Widget");
    build.define("drwav__read_and_close_s16", "Fundamentaldrwav__read_and_close_s16");
    build.define("modeldrwav__read_and_close_s16", "modelFundamentaldrwav__read_and_close_s16");
    build.define("drwav__read_and_close_s16Widget", "Fundamentaldrwav__read_and_close_s16Widget");
    build.define("drwav__read_and_close_s32", "Fundamentaldrwav__read_and_close_s32");
    build.define("modeldrwav__read_and_close_s32", "modelFundamentaldrwav__read_and_close_s32");
    build.define("drwav__read_and_close_s32Widget", "Fundamentaldrwav__read_and_close_s32Widget");
    build.define("drwav_alaw_to_f32", "Fundamentaldrwav_alaw_to_f32");
    build.define("modeldrwav_alaw_to_f32", "modelFundamentaldrwav_alaw_to_f32");
    build.define("drwav_alaw_to_f32Widget", "Fundamentaldrwav_alaw_to_f32Widget");
    build.define("drwav_alaw_to_s16", "Fundamentaldrwav_alaw_to_s16");
    build.define("modeldrwav_alaw_to_s16", "modelFundamentaldrwav_alaw_to_s16");
    build.define("drwav_alaw_to_s16Widget", "Fundamentaldrwav_alaw_to_s16Widget");
    build.define("drwav_alaw_to_s16", "Fundamentaldrwav_alaw_to_s16");
    build.define("modeldrwav_alaw_to_s16", "modelFundamentaldrwav_alaw_to_s16");
    build.define("drwav_alaw_to_s16Widget", "Fundamentaldrwav_alaw_to_s16Widget");
    build.define("drwav_alaw_to_s32", "Fundamentaldrwav_alaw_to_s32");
    build.define("modeldrwav_alaw_to_s32", "modelFundamentaldrwav_alaw_to_s32");
    build.define("drwav_alaw_to_s32Widget", "Fundamentaldrwav_alaw_to_s32Widget");
    build.define("drwav_bytes_to_f32", "Fundamentaldrwav_bytes_to_f32");
    build.define("modeldrwav_bytes_to_f32", "modelFundamentaldrwav_bytes_to_f32");
    build.define("drwav_bytes_to_f32Widget", "Fundamentaldrwav_bytes_to_f32Widget");
    build.define("drwav_bytes_to_s16", "Fundamentaldrwav_bytes_to_s16");
    build.define("modeldrwav_bytes_to_s16", "modelFundamentaldrwav_bytes_to_s16");
    build.define("drwav_bytes_to_s16Widget", "Fundamentaldrwav_bytes_to_s16Widget");
    build.define("drwav_bytes_to_s32", "Fundamentaldrwav_bytes_to_s32");
    build.define("modeldrwav_bytes_to_s32", "modelFundamentaldrwav_bytes_to_s32");
    build.define("drwav_bytes_to_s32Widget", "Fundamentaldrwav_bytes_to_s32Widget");
    build.define("drwav_bytes_to_s64", "Fundamentaldrwav_bytes_to_s64");
    build.define("modeldrwav_bytes_to_s64", "modelFundamentaldrwav_bytes_to_s64");
    build.define("drwav_bytes_to_s64Widget", "Fundamentaldrwav_bytes_to_s64Widget");
    build.define("drwav_bytes_to_u16", "Fundamentaldrwav_bytes_to_u16");
    build.define("modeldrwav_bytes_to_u16", "modelFundamentaldrwav_bytes_to_u16");
    build.define("drwav_bytes_to_u16Widget", "Fundamentaldrwav_bytes_to_u16Widget");
    build.define("drwav_bytes_to_u32", "Fundamentaldrwav_bytes_to_u32");
    build.define("modeldrwav_bytes_to_u32", "modelFundamentaldrwav_bytes_to_u32");
    build.define("drwav_bytes_to_u32Widget", "Fundamentaldrwav_bytes_to_u32Widget");
    build.define("drwav_bytes_to_u64", "Fundamentaldrwav_bytes_to_u64");
    build.define("modeldrwav_bytes_to_u64", "modelFundamentaldrwav_bytes_to_u64");
    build.define("drwav_bytes_to_u64Widget", "Fundamentaldrwav_bytes_to_u64Widget");
    build.define("drwav_close", "Fundamentaldrwav_close");
    build.define("modeldrwav_close", "modelFundamentaldrwav_close");
    build.define("drwav_closeWidget", "Fundamentaldrwav_closeWidget");
    build.define("drwav_close", "Fundamentaldrwav_close");
    build.define("modeldrwav_close", "modelFundamentaldrwav_close");
    build.define("drwav_closeWidget", "Fundamentaldrwav_closeWidget");
    build.define("drwav_container", "Fundamentaldrwav_container");
    build.define("modeldrwav_container", "modelFundamentaldrwav_container");
    build.define("drwav_containerWidget", "Fundamentaldrwav_containerWidget");
    build.define("drwav_data_chunk_size_riff", "Fundamentaldrwav_data_chunk_size_riff");
    build.define("modeldrwav_data_chunk_size_riff", "modelFundamentaldrwav_data_chunk_size_riff");
    build.define("drwav_data_chunk_size_riffWidget", "Fundamentaldrwav_data_chunk_size_riffWidget");
    build.define("drwav_data_chunk_size_w64", "Fundamentaldrwav_data_chunk_size_w64");
    build.define("modeldrwav_data_chunk_size_w64", "modelFundamentaldrwav_data_chunk_size_w64");
    build.define("drwav_data_chunk_size_w64Widget", "Fundamentaldrwav_data_chunk_size_w64Widget");
    build.define("drwav_data_format", "Fundamentaldrwav_data_format");
    build.define("modeldrwav_data_format", "modelFundamentaldrwav_data_format");
    build.define("drwav_data_formatWidget", "Fundamentaldrwav_data_formatWidget");
    build.define("drwav_f32_to_s16", "Fundamentaldrwav_f32_to_s16");
    build.define("modeldrwav_f32_to_s16", "modelFundamentaldrwav_f32_to_s16");
    build.define("drwav_f32_to_s16Widget", "Fundamentaldrwav_f32_to_s16Widget");
    build.define("drwav_f32_to_s32", "Fundamentaldrwav_f32_to_s32");
    build.define("modeldrwav_f32_to_s32", "modelFundamentaldrwav_f32_to_s32");
    build.define("drwav_f32_to_s32Widget", "Fundamentaldrwav_f32_to_s32Widget");
    build.define("drwav_f64_to_f32", "Fundamentaldrwav_f64_to_f32");
    build.define("modeldrwav_f64_to_f32", "modelFundamentaldrwav_f64_to_f32");
    build.define("drwav_f64_to_f32Widget", "Fundamentaldrwav_f64_to_f32Widget");
    build.define("drwav_f64_to_s16", "Fundamentaldrwav_f64_to_s16");
    build.define("modeldrwav_f64_to_s16", "modelFundamentaldrwav_f64_to_s16");
    build.define("drwav_f64_to_s16Widget", "Fundamentaldrwav_f64_to_s16Widget");
    build.define("drwav_f64_to_s16", "Fundamentaldrwav_f64_to_s16");
    build.define("modeldrwav_f64_to_s16", "modelFundamentaldrwav_f64_to_s16");
    build.define("drwav_f64_to_s16Widget", "Fundamentaldrwav_f64_to_s16Widget");
    build.define("drwav_f64_to_s32", "Fundamentaldrwav_f64_to_s32");
    build.define("modeldrwav_f64_to_s32", "modelFundamentaldrwav_f64_to_s32");
    build.define("drwav_f64_to_s32Widget", "Fundamentaldrwav_f64_to_s32Widget");
    build.define("drwav_fmt_get_format", "Fundamentaldrwav_fmt_get_format");
    build.define("modeldrwav_fmt_get_format", "modelFundamentaldrwav_fmt_get_format");
    build.define("drwav_fmt_get_formatWidget", "Fundamentaldrwav_fmt_get_formatWidget");
    build.define("drwav_fopen", "Fundamentaldrwav_fopen");
    build.define("modeldrwav_fopen", "modelFundamentaldrwav_fopen");
    build.define("drwav_fopenWidget", "Fundamentaldrwav_fopenWidget");
    build.define("drwav_fourcc_equal", "Fundamentaldrwav_fourcc_equal");
    build.define("modeldrwav_fourcc_equal", "modelFundamentaldrwav_fourcc_equal");
    build.define("drwav_fourcc_equalWidget", "Fundamentaldrwav_fourcc_equalWidget");
    build.define("drwav_free", "Fundamentaldrwav_free");
    build.define("modeldrwav_free", "modelFundamentaldrwav_free");
    build.define("drwav_freeWidget", "Fundamentaldrwav_freeWidget");
    build.define("drwav_get_cursor_in_pcm_frames", "Fundamentaldrwav_get_cursor_in_pcm_frames");
    build.define("modeldrwav_get_cursor_in_pcm_frames", "modelFundamentaldrwav_get_cursor_in_pcm_frames");
    build.define("drwav_get_cursor_in_pcm_framesWidget", "Fundamentaldrwav_get_cursor_in_pcm_framesWidget");
    build.define("drwav_get_length_in_pcm_frames", "Fundamentaldrwav_get_length_in_pcm_frames");
    build.define("modeldrwav_get_length_in_pcm_frames", "modelFundamentaldrwav_get_length_in_pcm_frames");
    build.define("drwav_get_length_in_pcm_framesWidget", "Fundamentaldrwav_get_length_in_pcm_framesWidget");
    build.define("drwav_guid_equal", "Fundamentaldrwav_guid_equal");
    build.define("modeldrwav_guid_equal", "modelFundamentaldrwav_guid_equal");
    build.define("drwav_guid_equalWidget", "Fundamentaldrwav_guid_equalWidget");
    build.define("drwav_init", "Fundamentaldrwav_init");
    build.define("modeldrwav_init", "modelFundamentaldrwav_init");
    build.define("drwav_initWidget", "Fundamentaldrwav_initWidget");
    build.define("drwav_init_ex", "Fundamentaldrwav_init_ex");
    build.define("modeldrwav_init_ex", "modelFundamentaldrwav_init_ex");
    build.define("drwav_init_exWidget", "Fundamentaldrwav_init_exWidget");
    build.define("drwav_init_file", "Fundamentaldrwav_init_file");
    build.define("modeldrwav_init_file", "modelFundamentaldrwav_init_file");
    build.define("drwav_init_fileWidget", "Fundamentaldrwav_init_fileWidget");
    build.define("drwav_init_file_ex", "Fundamentaldrwav_init_file_ex");
    build.define("modeldrwav_init_file_ex", "modelFundamentaldrwav_init_file_ex");
    build.define("drwav_init_file_exWidget", "Fundamentaldrwav_init_file_exWidget");
    build.define("drwav_init_file_ex_w", "Fundamentaldrwav_init_file_ex_w");
    build.define("modeldrwav_init_file_ex_w", "modelFundamentaldrwav_init_file_ex_w");
    build.define("drwav_init_file_ex_wWidget", "Fundamentaldrwav_init_file_ex_wWidget");
    build.define("drwav_init_file_w", "Fundamentaldrwav_init_file_w");
    build.define("modeldrwav_init_file_w", "modelFundamentaldrwav_init_file_w");
    build.define("drwav_init_file_wWidget", "Fundamentaldrwav_init_file_wWidget");
    build.define("drwav_init_file_with_metadata", "Fundamentaldrwav_init_file_with_metadata");
    build.define("modeldrwav_init_file_with_metadata", "modelFundamentaldrwav_init_file_with_metadata");
    build.define("drwav_init_file_with_metadataWidget", "Fundamentaldrwav_init_file_with_metadataWidget");
    build.define("drwav_init_file_with_metadata_w", "Fundamentaldrwav_init_file_with_metadata_w");
    build.define("modeldrwav_init_file_with_metadata_w", "modelFundamentaldrwav_init_file_with_metadata_w");
    build.define("drwav_init_file_with_metadata_wWidget", "Fundamentaldrwav_init_file_with_metadata_wWidget");
    build.define("drwav_init_file_write", "Fundamentaldrwav_init_file_write");
    build.define("modeldrwav_init_file_write", "modelFundamentaldrwav_init_file_write");
    build.define("drwav_init_file_writeWidget", "Fundamentaldrwav_init_file_writeWidget");
    build.define("drwav_init_file_write", "Fundamentaldrwav_init_file_write");
    build.define("modeldrwav_init_file_write", "modelFundamentaldrwav_init_file_write");
    build.define("drwav_init_file_writeWidget", "Fundamentaldrwav_init_file_writeWidget");
    build.define("drwav_init_file_write__internal", "Fundamentaldrwav_init_file_write__internal");
    build.define("modeldrwav_init_file_write__internal", "modelFundamentaldrwav_init_file_write__internal");
    build.define("drwav_init_file_write__internalWidget", "Fundamentaldrwav_init_file_write__internalWidget");
    build.define("drwav_init_file_write__internal", "Fundamentaldrwav_init_file_write__internal");
    build.define("modeldrwav_init_file_write__internal", "modelFundamentaldrwav_init_file_write__internal");
    build.define("drwav_init_file_write__internalWidget", "Fundamentaldrwav_init_file_write__internalWidget");
    build.define("drwav_init_file_write_sequential", "Fundamentaldrwav_init_file_write_sequential");
    build.define("modeldrwav_init_file_write_sequential", "modelFundamentaldrwav_init_file_write_sequential");
    build.define("drwav_init_file_write_sequentialWidget", "Fundamentaldrwav_init_file_write_sequentialWidget");
    build.define("drwav_init_file_write_sequential", "Fundamentaldrwav_init_file_write_sequential");
    build.define("modeldrwav_init_file_write_sequential", "modelFundamentaldrwav_init_file_write_sequential");
    build.define("drwav_init_file_write_sequentialWidget", "Fundamentaldrwav_init_file_write_sequentialWidget");
    build.define("drwav_init_file_write_sequential_pcm_frames", "Fundamentaldrwav_init_file_write_sequential_pcm_frames");
    build.define("modeldrwav_init_file_write_sequential_pcm_frames", "modelFundamentaldrwav_init_file_write_sequential_pcm_frames");
    build.define("drwav_init_file_write_sequential_pcm_framesWidget", "Fundamentaldrwav_init_file_write_sequential_pcm_framesWidget");
    build.define("drwav_init_file_write_sequential_pcm_frames_w", "Fundamentaldrwav_init_file_write_sequential_pcm_frames_w");
    build.define("modeldrwav_init_file_write_sequential_pcm_frames_w", "modelFundamentaldrwav_init_file_write_sequential_pcm_frames_w");
    build.define("drwav_init_file_write_sequential_pcm_frames_wWidget", "Fundamentaldrwav_init_file_write_sequential_pcm_frames_wWidget");
    build.define("drwav_init_file_write_sequential_w", "Fundamentaldrwav_init_file_write_sequential_w");
    build.define("modeldrwav_init_file_write_sequential_w", "modelFundamentaldrwav_init_file_write_sequential_w");
    build.define("drwav_init_file_write_sequential_wWidget", "Fundamentaldrwav_init_file_write_sequential_wWidget");
    build.define("drwav_init_file_write_w", "Fundamentaldrwav_init_file_write_w");
    build.define("modeldrwav_init_file_write_w", "modelFundamentaldrwav_init_file_write_w");
    build.define("drwav_init_file_write_wWidget", "Fundamentaldrwav_init_file_write_wWidget");
    build.define("drwav_init_memory", "Fundamentaldrwav_init_memory");
    build.define("modeldrwav_init_memory", "modelFundamentaldrwav_init_memory");
    build.define("drwav_init_memoryWidget", "Fundamentaldrwav_init_memoryWidget");
    build.define("drwav_init_memory_ex", "Fundamentaldrwav_init_memory_ex");
    build.define("modeldrwav_init_memory_ex", "modelFundamentaldrwav_init_memory_ex");
    build.define("drwav_init_memory_exWidget", "Fundamentaldrwav_init_memory_exWidget");
    build.define("drwav_init_memory_with_metadata", "Fundamentaldrwav_init_memory_with_metadata");
    build.define("modeldrwav_init_memory_with_metadata", "modelFundamentaldrwav_init_memory_with_metadata");
    build.define("drwav_init_memory_with_metadataWidget", "Fundamentaldrwav_init_memory_with_metadataWidget");
    build.define("drwav_init_memory_write", "Fundamentaldrwav_init_memory_write");
    build.define("modeldrwav_init_memory_write", "modelFundamentaldrwav_init_memory_write");
    build.define("drwav_init_memory_writeWidget", "Fundamentaldrwav_init_memory_writeWidget");
    build.define("drwav_init_memory_write", "Fundamentaldrwav_init_memory_write");
    build.define("modeldrwav_init_memory_write", "modelFundamentaldrwav_init_memory_write");
    build.define("drwav_init_memory_writeWidget", "Fundamentaldrwav_init_memory_writeWidget");
    build.define("drwav_init_memory_write__internal", "Fundamentaldrwav_init_memory_write__internal");
    build.define("modeldrwav_init_memory_write__internal", "modelFundamentaldrwav_init_memory_write__internal");
    build.define("drwav_init_memory_write__internalWidget", "Fundamentaldrwav_init_memory_write__internalWidget");
    build.define("drwav_init_memory_write__internal", "Fundamentaldrwav_init_memory_write__internal");
    build.define("modeldrwav_init_memory_write__internal", "modelFundamentaldrwav_init_memory_write__internal");
    build.define("drwav_init_memory_write__internalWidget", "Fundamentaldrwav_init_memory_write__internalWidget");
    build.define("drwav_init_memory_write_sequential", "Fundamentaldrwav_init_memory_write_sequential");
    build.define("modeldrwav_init_memory_write_sequential", "modelFundamentaldrwav_init_memory_write_sequential");
    build.define("drwav_init_memory_write_sequentialWidget", "Fundamentaldrwav_init_memory_write_sequentialWidget");
    build.define("drwav_init_memory_write_sequential_pcm_frames", "Fundamentaldrwav_init_memory_write_sequential_pcm_frames");
    build.define("modeldrwav_init_memory_write_sequential_pcm_frames", "modelFundamentaldrwav_init_memory_write_sequential_pcm_frames");
    build.define("drwav_init_memory_write_sequential_pcm_framesWidget", "Fundamentaldrwav_init_memory_write_sequential_pcm_framesWidget");
    build.define("drwav_init_with_metadata", "Fundamentaldrwav_init_with_metadata");
    build.define("modeldrwav_init_with_metadata", "modelFundamentaldrwav_init_with_metadata");
    build.define("drwav_init_with_metadataWidget", "Fundamentaldrwav_init_with_metadataWidget");
    build.define("drwav_init_write", "Fundamentaldrwav_init_write");
    build.define("modeldrwav_init_write", "modelFundamentaldrwav_init_write");
    build.define("drwav_init_writeWidget", "Fundamentaldrwav_init_writeWidget");
    build.define("drwav_init_write", "Fundamentaldrwav_init_write");
    build.define("modeldrwav_init_write", "modelFundamentaldrwav_init_write");
    build.define("drwav_init_writeWidget", "Fundamentaldrwav_init_writeWidget");
    build.define("drwav_init_write__internal", "Fundamentaldrwav_init_write__internal");
    build.define("modeldrwav_init_write__internal", "modelFundamentaldrwav_init_write__internal");
    build.define("drwav_init_write__internalWidget", "Fundamentaldrwav_init_write__internalWidget");
    build.define("drwav_init_write_sequential", "Fundamentaldrwav_init_write_sequential");
    build.define("modeldrwav_init_write_sequential", "modelFundamentaldrwav_init_write_sequential");
    build.define("drwav_init_write_sequentialWidget", "Fundamentaldrwav_init_write_sequentialWidget");
    build.define("drwav_init_write_sequential_pcm_frames", "Fundamentaldrwav_init_write_sequential_pcm_frames");
    build.define("modeldrwav_init_write_sequential_pcm_frames", "modelFundamentaldrwav_init_write_sequential_pcm_frames");
    build.define("drwav_init_write_sequential_pcm_framesWidget", "Fundamentaldrwav_init_write_sequential_pcm_framesWidget");
    build.define("drwav_init_write_with_metadata", "Fundamentaldrwav_init_write_with_metadata");
    build.define("modeldrwav_init_write_with_metadata", "modelFundamentaldrwav_init_write_with_metadata");
    build.define("drwav_init_write_with_metadataWidget", "Fundamentaldrwav_init_write_with_metadataWidget");
    build.define("drwav_metadata", "Fundamentaldrwav_metadata");
    build.define("modeldrwav_metadata", "modelFundamentaldrwav_metadata");
    build.define("drwav_metadataWidget", "Fundamentaldrwav_metadataWidget");
    build.define("drwav__metadata_parser", "Fundamentaldrwav__metadata_parser");
    build.define("modeldrwav__metadata_parser", "modelFundamentaldrwav__metadata_parser");
    build.define("drwav__metadata_parserWidget", "Fundamentaldrwav__metadata_parserWidget");
    build.define("drwav_mulaw_to_f32", "Fundamentaldrwav_mulaw_to_f32");
    build.define("modeldrwav_mulaw_to_f32", "modelFundamentaldrwav_mulaw_to_f32");
    build.define("drwav_mulaw_to_f32Widget", "Fundamentaldrwav_mulaw_to_f32Widget");
    build.define("drwav_mulaw_to_s16", "Fundamentaldrwav_mulaw_to_s16");
    build.define("modeldrwav_mulaw_to_s16", "modelFundamentaldrwav_mulaw_to_s16");
    build.define("drwav_mulaw_to_s16Widget", "Fundamentaldrwav_mulaw_to_s16Widget");
    build.define("drwav_mulaw_to_s16", "Fundamentaldrwav_mulaw_to_s16");
    build.define("modeldrwav_mulaw_to_s16", "modelFundamentaldrwav_mulaw_to_s16");
    build.define("drwav_mulaw_to_s16Widget", "Fundamentaldrwav_mulaw_to_s16Widget");
    build.define("drwav_mulaw_to_s32", "Fundamentaldrwav_mulaw_to_s32");
    build.define("modeldrwav_mulaw_to_s32", "modelFundamentaldrwav_mulaw_to_s32");
    build.define("drwav_mulaw_to_s32Widget", "Fundamentaldrwav_mulaw_to_s32Widget");
    build.define("drwav_open", "Fundamentaldrwav_open");
    build.define("modeldrwav_open", "modelFundamentaldrwav_open");
    build.define("drwav_openWidget", "Fundamentaldrwav_openWidget");
    build.define("drwav_open_and_read_f32", "Fundamentaldrwav_open_and_read_f32");
    build.define("modeldrwav_open_and_read_f32", "modelFundamentaldrwav_open_and_read_f32");
    build.define("drwav_open_and_read_f32Widget", "Fundamentaldrwav_open_and_read_f32Widget");
    build.define("drwav_open_and_read_file_f32", "Fundamentaldrwav_open_and_read_file_f32");
    build.define("modeldrwav_open_and_read_file_f32", "modelFundamentaldrwav_open_and_read_file_f32");
    build.define("drwav_open_and_read_file_f32Widget", "Fundamentaldrwav_open_and_read_file_f32Widget");
    build.define("drwav_open_and_read_file_s16", "Fundamentaldrwav_open_and_read_file_s16");
    build.define("modeldrwav_open_and_read_file_s16", "modelFundamentaldrwav_open_and_read_file_s16");
    build.define("drwav_open_and_read_file_s16Widget", "Fundamentaldrwav_open_and_read_file_s16Widget");
    build.define("drwav_open_and_read_file_s32", "Fundamentaldrwav_open_and_read_file_s32");
    build.define("modeldrwav_open_and_read_file_s32", "modelFundamentaldrwav_open_and_read_file_s32");
    build.define("drwav_open_and_read_file_s32Widget", "Fundamentaldrwav_open_and_read_file_s32Widget");
    build.define("drwav_open_and_read_memory_f32", "Fundamentaldrwav_open_and_read_memory_f32");
    build.define("modeldrwav_open_and_read_memory_f32", "modelFundamentaldrwav_open_and_read_memory_f32");
    build.define("drwav_open_and_read_memory_f32Widget", "Fundamentaldrwav_open_and_read_memory_f32Widget");
    build.define("drwav_open_and_read_memory_s16", "Fundamentaldrwav_open_and_read_memory_s16");
    build.define("modeldrwav_open_and_read_memory_s16", "modelFundamentaldrwav_open_and_read_memory_s16");
    build.define("drwav_open_and_read_memory_s16Widget", "Fundamentaldrwav_open_and_read_memory_s16Widget");
    build.define("drwav_open_and_read_memory_s32", "Fundamentaldrwav_open_and_read_memory_s32");
    build.define("modeldrwav_open_and_read_memory_s32", "modelFundamentaldrwav_open_and_read_memory_s32");
    build.define("drwav_open_and_read_memory_s32Widget", "Fundamentaldrwav_open_and_read_memory_s32Widget");
    build.define("drwav_open_and_read_pcm_frames_f32", "Fundamentaldrwav_open_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_and_read_pcm_frames_f32", "modelFundamentaldrwav_open_and_read_pcm_frames_f32");
    build.define("drwav_open_and_read_pcm_frames_f32Widget", "Fundamentaldrwav_open_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_and_read_pcm_frames_s16", "Fundamentaldrwav_open_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_and_read_pcm_frames_s16", "modelFundamentaldrwav_open_and_read_pcm_frames_s16");
    build.define("drwav_open_and_read_pcm_frames_s16Widget", "Fundamentaldrwav_open_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_and_read_pcm_frames_s32", "Fundamentaldrwav_open_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_and_read_pcm_frames_s32", "modelFundamentaldrwav_open_and_read_pcm_frames_s32");
    build.define("drwav_open_and_read_pcm_frames_s32Widget", "Fundamentaldrwav_open_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_and_read_s16", "Fundamentaldrwav_open_and_read_s16");
    build.define("modeldrwav_open_and_read_s16", "modelFundamentaldrwav_open_and_read_s16");
    build.define("drwav_open_and_read_s16Widget", "Fundamentaldrwav_open_and_read_s16Widget");
    build.define("drwav_open_and_read_s32", "Fundamentaldrwav_open_and_read_s32");
    build.define("modeldrwav_open_and_read_s32", "modelFundamentaldrwav_open_and_read_s32");
    build.define("drwav_open_and_read_s32Widget", "Fundamentaldrwav_open_and_read_s32Widget");
    build.define("drwav_open_ex", "Fundamentaldrwav_open_ex");
    build.define("modeldrwav_open_ex", "modelFundamentaldrwav_open_ex");
    build.define("drwav_open_exWidget", "Fundamentaldrwav_open_exWidget");
    build.define("drwav_open_file", "Fundamentaldrwav_open_file");
    build.define("modeldrwav_open_file", "modelFundamentaldrwav_open_file");
    build.define("drwav_open_fileWidget", "Fundamentaldrwav_open_fileWidget");
    build.define("drwav_open_file_and_read_f32", "Fundamentaldrwav_open_file_and_read_f32");
    build.define("modeldrwav_open_file_and_read_f32", "modelFundamentaldrwav_open_file_and_read_f32");
    build.define("drwav_open_file_and_read_f32Widget", "Fundamentaldrwav_open_file_and_read_f32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_f32", "Fundamentaldrwav_open_file_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_file_and_read_pcm_frames_f32", "modelFundamentaldrwav_open_file_and_read_pcm_frames_f32");
    build.define("drwav_open_file_and_read_pcm_frames_f32Widget", "Fundamentaldrwav_open_file_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_f32_w", "Fundamentaldrwav_open_file_and_read_pcm_frames_f32_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_f32_w", "modelFundamentaldrwav_open_file_and_read_pcm_frames_f32_w");
    build.define("drwav_open_file_and_read_pcm_frames_f32_wWidget", "Fundamentaldrwav_open_file_and_read_pcm_frames_f32_wWidget");
    build.define("drwav_open_file_and_read_pcm_frames_s16", "Fundamentaldrwav_open_file_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s16", "modelFundamentaldrwav_open_file_and_read_pcm_frames_s16");
    build.define("drwav_open_file_and_read_pcm_frames_s16Widget", "Fundamentaldrwav_open_file_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_file_and_read_pcm_frames_s16_w", "Fundamentaldrwav_open_file_and_read_pcm_frames_s16_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s16_w", "modelFundamentaldrwav_open_file_and_read_pcm_frames_s16_w");
    build.define("drwav_open_file_and_read_pcm_frames_s16_wWidget", "Fundamentaldrwav_open_file_and_read_pcm_frames_s16_wWidget");
    build.define("drwav_open_file_and_read_pcm_frames_s32", "Fundamentaldrwav_open_file_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s32", "modelFundamentaldrwav_open_file_and_read_pcm_frames_s32");
    build.define("drwav_open_file_and_read_pcm_frames_s32Widget", "Fundamentaldrwav_open_file_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_s32_w", "Fundamentaldrwav_open_file_and_read_pcm_frames_s32_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s32_w", "modelFundamentaldrwav_open_file_and_read_pcm_frames_s32_w");
    build.define("drwav_open_file_and_read_pcm_frames_s32_wWidget", "Fundamentaldrwav_open_file_and_read_pcm_frames_s32_wWidget");
    build.define("drwav_open_file_and_read_s16", "Fundamentaldrwav_open_file_and_read_s16");
    build.define("modeldrwav_open_file_and_read_s16", "modelFundamentaldrwav_open_file_and_read_s16");
    build.define("drwav_open_file_and_read_s16Widget", "Fundamentaldrwav_open_file_and_read_s16Widget");
    build.define("drwav_open_file_and_read_s32", "Fundamentaldrwav_open_file_and_read_s32");
    build.define("modeldrwav_open_file_and_read_s32", "modelFundamentaldrwav_open_file_and_read_s32");
    build.define("drwav_open_file_and_read_s32Widget", "Fundamentaldrwav_open_file_and_read_s32Widget");
    build.define("drwav_open_file_ex", "Fundamentaldrwav_open_file_ex");
    build.define("modeldrwav_open_file_ex", "modelFundamentaldrwav_open_file_ex");
    build.define("drwav_open_file_exWidget", "Fundamentaldrwav_open_file_exWidget");
    build.define("drwav_open_file_write", "Fundamentaldrwav_open_file_write");
    build.define("modeldrwav_open_file_write", "modelFundamentaldrwav_open_file_write");
    build.define("drwav_open_file_writeWidget", "Fundamentaldrwav_open_file_writeWidget");
    build.define("drwav_open_file_write", "Fundamentaldrwav_open_file_write");
    build.define("modeldrwav_open_file_write", "modelFundamentaldrwav_open_file_write");
    build.define("drwav_open_file_writeWidget", "Fundamentaldrwav_open_file_writeWidget");
    build.define("drwav_open_file_write__internal", "Fundamentaldrwav_open_file_write__internal");
    build.define("modeldrwav_open_file_write__internal", "modelFundamentaldrwav_open_file_write__internal");
    build.define("drwav_open_file_write__internalWidget", "Fundamentaldrwav_open_file_write__internalWidget");
    build.define("drwav_open_file_write__internal", "Fundamentaldrwav_open_file_write__internal");
    build.define("modeldrwav_open_file_write__internal", "modelFundamentaldrwav_open_file_write__internal");
    build.define("drwav_open_file_write__internalWidget", "Fundamentaldrwav_open_file_write__internalWidget");
    build.define("drwav_open_file_write_sequential", "Fundamentaldrwav_open_file_write_sequential");
    build.define("modeldrwav_open_file_write_sequential", "modelFundamentaldrwav_open_file_write_sequential");
    build.define("drwav_open_file_write_sequentialWidget", "Fundamentaldrwav_open_file_write_sequentialWidget");
    build.define("drwav_open_file_write_sequential", "Fundamentaldrwav_open_file_write_sequential");
    build.define("modeldrwav_open_file_write_sequential", "modelFundamentaldrwav_open_file_write_sequential");
    build.define("drwav_open_file_write_sequentialWidget", "Fundamentaldrwav_open_file_write_sequentialWidget");
    build.define("drwav_open_memory", "Fundamentaldrwav_open_memory");
    build.define("modeldrwav_open_memory", "modelFundamentaldrwav_open_memory");
    build.define("drwav_open_memoryWidget", "Fundamentaldrwav_open_memoryWidget");
    build.define("drwav_open_memory_and_read_f32", "Fundamentaldrwav_open_memory_and_read_f32");
    build.define("modeldrwav_open_memory_and_read_f32", "modelFundamentaldrwav_open_memory_and_read_f32");
    build.define("drwav_open_memory_and_read_f32Widget", "Fundamentaldrwav_open_memory_and_read_f32Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_f32", "Fundamentaldrwav_open_memory_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_f32", "modelFundamentaldrwav_open_memory_and_read_pcm_frames_f32");
    build.define("drwav_open_memory_and_read_pcm_frames_f32Widget", "Fundamentaldrwav_open_memory_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_s16", "Fundamentaldrwav_open_memory_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_s16", "modelFundamentaldrwav_open_memory_and_read_pcm_frames_s16");
    build.define("drwav_open_memory_and_read_pcm_frames_s16Widget", "Fundamentaldrwav_open_memory_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_s32", "Fundamentaldrwav_open_memory_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_s32", "modelFundamentaldrwav_open_memory_and_read_pcm_frames_s32");
    build.define("drwav_open_memory_and_read_pcm_frames_s32Widget", "Fundamentaldrwav_open_memory_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_memory_and_read_s16", "Fundamentaldrwav_open_memory_and_read_s16");
    build.define("modeldrwav_open_memory_and_read_s16", "modelFundamentaldrwav_open_memory_and_read_s16");
    build.define("drwav_open_memory_and_read_s16Widget", "Fundamentaldrwav_open_memory_and_read_s16Widget");
    build.define("drwav_open_memory_and_read_s32", "Fundamentaldrwav_open_memory_and_read_s32");
    build.define("modeldrwav_open_memory_and_read_s32", "modelFundamentaldrwav_open_memory_and_read_s32");
    build.define("drwav_open_memory_and_read_s32Widget", "Fundamentaldrwav_open_memory_and_read_s32Widget");
    build.define("drwav_open_memory_ex", "Fundamentaldrwav_open_memory_ex");
    build.define("modeldrwav_open_memory_ex", "modelFundamentaldrwav_open_memory_ex");
    build.define("drwav_open_memory_exWidget", "Fundamentaldrwav_open_memory_exWidget");
    build.define("drwav_open_memory_write", "Fundamentaldrwav_open_memory_write");
    build.define("modeldrwav_open_memory_write", "modelFundamentaldrwav_open_memory_write");
    build.define("drwav_open_memory_writeWidget", "Fundamentaldrwav_open_memory_writeWidget");
    build.define("drwav_open_memory_write", "Fundamentaldrwav_open_memory_write");
    build.define("modeldrwav_open_memory_write", "modelFundamentaldrwav_open_memory_write");
    build.define("drwav_open_memory_writeWidget", "Fundamentaldrwav_open_memory_writeWidget");
    build.define("drwav_open_memory_write__internal", "Fundamentaldrwav_open_memory_write__internal");
    build.define("modeldrwav_open_memory_write__internal", "modelFundamentaldrwav_open_memory_write__internal");
    build.define("drwav_open_memory_write__internalWidget", "Fundamentaldrwav_open_memory_write__internalWidget");
    build.define("drwav_open_memory_write__internal", "Fundamentaldrwav_open_memory_write__internal");
    build.define("modeldrwav_open_memory_write__internal", "modelFundamentaldrwav_open_memory_write__internal");
    build.define("drwav_open_memory_write__internalWidget", "Fundamentaldrwav_open_memory_write__internalWidget");
    build.define("drwav_open_memory_write_sequential", "Fundamentaldrwav_open_memory_write_sequential");
    build.define("modeldrwav_open_memory_write_sequential", "modelFundamentaldrwav_open_memory_write_sequential");
    build.define("drwav_open_memory_write_sequentialWidget", "Fundamentaldrwav_open_memory_write_sequentialWidget");
    build.define("drwav_open_write", "Fundamentaldrwav_open_write");
    build.define("modeldrwav_open_write", "modelFundamentaldrwav_open_write");
    build.define("drwav_open_writeWidget", "Fundamentaldrwav_open_writeWidget");
    build.define("drwav_open_write", "Fundamentaldrwav_open_write");
    build.define("modeldrwav_open_write", "modelFundamentaldrwav_open_write");
    build.define("drwav_open_writeWidget", "Fundamentaldrwav_open_writeWidget");
    build.define("drwav_open_write__internal", "Fundamentaldrwav_open_write__internal");
    build.define("modeldrwav_open_write__internal", "modelFundamentaldrwav_open_write__internal");
    build.define("drwav_open_write__internalWidget", "Fundamentaldrwav_open_write__internalWidget");
    build.define("drwav_open_write_sequential", "Fundamentaldrwav_open_write_sequential");
    build.define("modeldrwav_open_write_sequential", "modelFundamentaldrwav_open_write_sequential");
    build.define("drwav_open_write_sequentialWidget", "Fundamentaldrwav_open_write_sequentialWidget");
    build.define("drwav_read", "Fundamentaldrwav_read");
    build.define("modeldrwav_read", "modelFundamentaldrwav_read");
    build.define("drwav_readWidget", "Fundamentaldrwav_readWidget");
    build.define("drwav_read_f32", "Fundamentaldrwav_read_f32");
    build.define("modeldrwav_read_f32", "modelFundamentaldrwav_read_f32");
    build.define("drwav_read_f32Widget", "Fundamentaldrwav_read_f32Widget");
    build.define("drwav_read_f32__alaw", "Fundamentaldrwav_read_f32__alaw");
    build.define("modeldrwav_read_f32__alaw", "modelFundamentaldrwav_read_f32__alaw");
    build.define("drwav_read_f32__alawWidget", "Fundamentaldrwav_read_f32__alawWidget");
    build.define("drwav_read_f32__alaw", "Fundamentaldrwav_read_f32__alaw");
    build.define("modeldrwav_read_f32__alaw", "modelFundamentaldrwav_read_f32__alaw");
    build.define("drwav_read_f32__alawWidget", "Fundamentaldrwav_read_f32__alawWidget");
    build.define("drwav_read_f32__ieee", "Fundamentaldrwav_read_f32__ieee");
    build.define("modeldrwav_read_f32__ieee", "modelFundamentaldrwav_read_f32__ieee");
    build.define("drwav_read_f32__ieeeWidget", "Fundamentaldrwav_read_f32__ieeeWidget");
    build.define("drwav_read_f32__ieee", "Fundamentaldrwav_read_f32__ieee");
    build.define("modeldrwav_read_f32__ieee", "modelFundamentaldrwav_read_f32__ieee");
    build.define("drwav_read_f32__ieeeWidget", "Fundamentaldrwav_read_f32__ieeeWidget");
    build.define("drwav_read_f32__ima", "Fundamentaldrwav_read_f32__ima");
    build.define("modeldrwav_read_f32__ima", "modelFundamentaldrwav_read_f32__ima");
    build.define("drwav_read_f32__imaWidget", "Fundamentaldrwav_read_f32__imaWidget");
    build.define("drwav_read_f32__ima", "Fundamentaldrwav_read_f32__ima");
    build.define("modeldrwav_read_f32__ima", "modelFundamentaldrwav_read_f32__ima");
    build.define("drwav_read_f32__imaWidget", "Fundamentaldrwav_read_f32__imaWidget");
    build.define("drwav_read_f32__msadpcm", "Fundamentaldrwav_read_f32__msadpcm");
    build.define("modeldrwav_read_f32__msadpcm", "modelFundamentaldrwav_read_f32__msadpcm");
    build.define("drwav_read_f32__msadpcmWidget", "Fundamentaldrwav_read_f32__msadpcmWidget");
    build.define("drwav_read_f32__msadpcm", "Fundamentaldrwav_read_f32__msadpcm");
    build.define("modeldrwav_read_f32__msadpcm", "modelFundamentaldrwav_read_f32__msadpcm");
    build.define("drwav_read_f32__msadpcmWidget", "Fundamentaldrwav_read_f32__msadpcmWidget");
    build.define("drwav_read_f32__mulaw", "Fundamentaldrwav_read_f32__mulaw");
    build.define("modeldrwav_read_f32__mulaw", "modelFundamentaldrwav_read_f32__mulaw");
    build.define("drwav_read_f32__mulawWidget", "Fundamentaldrwav_read_f32__mulawWidget");
    build.define("drwav_read_f32__mulaw", "Fundamentaldrwav_read_f32__mulaw");
    build.define("modeldrwav_read_f32__mulaw", "modelFundamentaldrwav_read_f32__mulaw");
    build.define("drwav_read_f32__mulawWidget", "Fundamentaldrwav_read_f32__mulawWidget");
    build.define("drwav_read_f32__pcm", "Fundamentaldrwav_read_f32__pcm");
    build.define("modeldrwav_read_f32__pcm", "modelFundamentaldrwav_read_f32__pcm");
    build.define("drwav_read_f32__pcmWidget", "Fundamentaldrwav_read_f32__pcmWidget");
    build.define("drwav_read_f32__pcm", "Fundamentaldrwav_read_f32__pcm");
    build.define("modeldrwav_read_f32__pcm", "modelFundamentaldrwav_read_f32__pcm");
    build.define("drwav_read_f32__pcmWidget", "Fundamentaldrwav_read_f32__pcmWidget");
    build.define("drwav_read_pcm_frames", "Fundamentaldrwav_read_pcm_frames");
    build.define("modeldrwav_read_pcm_frames", "modelFundamentaldrwav_read_pcm_frames");
    build.define("drwav_read_pcm_framesWidget", "Fundamentaldrwav_read_pcm_framesWidget");
    build.define("drwav_read_pcm_frames_be", "Fundamentaldrwav_read_pcm_frames_be");
    build.define("modeldrwav_read_pcm_frames_be", "modelFundamentaldrwav_read_pcm_frames_be");
    build.define("drwav_read_pcm_frames_beWidget", "Fundamentaldrwav_read_pcm_frames_beWidget");
    build.define("drwav_read_pcm_frames_f32", "Fundamentaldrwav_read_pcm_frames_f32");
    build.define("modeldrwav_read_pcm_frames_f32", "modelFundamentaldrwav_read_pcm_frames_f32");
    build.define("drwav_read_pcm_frames_f32Widget", "Fundamentaldrwav_read_pcm_frames_f32Widget");
    build.define("drwav_read_pcm_frames_f32be", "Fundamentaldrwav_read_pcm_frames_f32be");
    build.define("modeldrwav_read_pcm_frames_f32be", "modelFundamentaldrwav_read_pcm_frames_f32be");
    build.define("drwav_read_pcm_frames_f32beWidget", "Fundamentaldrwav_read_pcm_frames_f32beWidget");
    build.define("drwav_read_pcm_frames_f32le", "Fundamentaldrwav_read_pcm_frames_f32le");
    build.define("modeldrwav_read_pcm_frames_f32le", "modelFundamentaldrwav_read_pcm_frames_f32le");
    build.define("drwav_read_pcm_frames_f32leWidget", "Fundamentaldrwav_read_pcm_frames_f32leWidget");
    build.define("drwav_read_pcm_frames_le", "Fundamentaldrwav_read_pcm_frames_le");
    build.define("modeldrwav_read_pcm_frames_le", "modelFundamentaldrwav_read_pcm_frames_le");
    build.define("drwav_read_pcm_frames_leWidget", "Fundamentaldrwav_read_pcm_frames_leWidget");
    build.define("drwav_read_pcm_frames_s16", "Fundamentaldrwav_read_pcm_frames_s16");
    build.define("modeldrwav_read_pcm_frames_s16", "modelFundamentaldrwav_read_pcm_frames_s16");
    build.define("drwav_read_pcm_frames_s16Widget", "Fundamentaldrwav_read_pcm_frames_s16Widget");
    build.define("drwav_read_pcm_frames_s16be", "Fundamentaldrwav_read_pcm_frames_s16be");
    build.define("modeldrwav_read_pcm_frames_s16be", "modelFundamentaldrwav_read_pcm_frames_s16be");
    build.define("drwav_read_pcm_frames_s16beWidget", "Fundamentaldrwav_read_pcm_frames_s16beWidget");
    build.define("drwav_read_pcm_frames_s16le", "Fundamentaldrwav_read_pcm_frames_s16le");
    build.define("modeldrwav_read_pcm_frames_s16le", "modelFundamentaldrwav_read_pcm_frames_s16le");
    build.define("drwav_read_pcm_frames_s16leWidget", "Fundamentaldrwav_read_pcm_frames_s16leWidget");
    build.define("drwav_read_pcm_frames_s32", "Fundamentaldrwav_read_pcm_frames_s32");
    build.define("modeldrwav_read_pcm_frames_s32", "modelFundamentaldrwav_read_pcm_frames_s32");
    build.define("drwav_read_pcm_frames_s32Widget", "Fundamentaldrwav_read_pcm_frames_s32Widget");
    build.define("drwav_read_pcm_frames_s32be", "Fundamentaldrwav_read_pcm_frames_s32be");
    build.define("modeldrwav_read_pcm_frames_s32be", "modelFundamentaldrwav_read_pcm_frames_s32be");
    build.define("drwav_read_pcm_frames_s32beWidget", "Fundamentaldrwav_read_pcm_frames_s32beWidget");
    build.define("drwav_read_pcm_frames_s32le", "Fundamentaldrwav_read_pcm_frames_s32le");
    build.define("modeldrwav_read_pcm_frames_s32le", "modelFundamentaldrwav_read_pcm_frames_s32le");
    build.define("drwav_read_pcm_frames_s32leWidget", "Fundamentaldrwav_read_pcm_frames_s32leWidget");
    build.define("drwav_read_raw", "Fundamentaldrwav_read_raw");
    build.define("modeldrwav_read_raw", "modelFundamentaldrwav_read_raw");
    build.define("drwav_read_rawWidget", "Fundamentaldrwav_read_rawWidget");
    build.define("drwav_read_s16", "Fundamentaldrwav_read_s16");
    build.define("modeldrwav_read_s16", "modelFundamentaldrwav_read_s16");
    build.define("drwav_read_s16Widget", "Fundamentaldrwav_read_s16Widget");
    build.define("drwav_read_s16__alaw", "Fundamentaldrwav_read_s16__alaw");
    build.define("modeldrwav_read_s16__alaw", "modelFundamentaldrwav_read_s16__alaw");
    build.define("drwav_read_s16__alawWidget", "Fundamentaldrwav_read_s16__alawWidget");
    build.define("drwav_read_s16__ieee", "Fundamentaldrwav_read_s16__ieee");
    build.define("modeldrwav_read_s16__ieee", "modelFundamentaldrwav_read_s16__ieee");
    build.define("drwav_read_s16__ieeeWidget", "Fundamentaldrwav_read_s16__ieeeWidget");
    build.define("drwav_read_s16__ima", "Fundamentaldrwav_read_s16__ima");
    build.define("modeldrwav_read_s16__ima", "modelFundamentaldrwav_read_s16__ima");
    build.define("drwav_read_s16__imaWidget", "Fundamentaldrwav_read_s16__imaWidget");
    build.define("drwav_read_s16__msadpcm", "Fundamentaldrwav_read_s16__msadpcm");
    build.define("modeldrwav_read_s16__msadpcm", "modelFundamentaldrwav_read_s16__msadpcm");
    build.define("drwav_read_s16__msadpcmWidget", "Fundamentaldrwav_read_s16__msadpcmWidget");
    build.define("drwav_read_s16__mulaw", "Fundamentaldrwav_read_s16__mulaw");
    build.define("modeldrwav_read_s16__mulaw", "modelFundamentaldrwav_read_s16__mulaw");
    build.define("drwav_read_s16__mulawWidget", "Fundamentaldrwav_read_s16__mulawWidget");
    build.define("drwav_read_s16__pcm", "Fundamentaldrwav_read_s16__pcm");
    build.define("modeldrwav_read_s16__pcm", "modelFundamentaldrwav_read_s16__pcm");
    build.define("drwav_read_s16__pcmWidget", "Fundamentaldrwav_read_s16__pcmWidget");
    build.define("drwav_read_s32", "Fundamentaldrwav_read_s32");
    build.define("modeldrwav_read_s32", "modelFundamentaldrwav_read_s32");
    build.define("drwav_read_s32Widget", "Fundamentaldrwav_read_s32Widget");
    build.define("drwav_read_s32__alaw", "Fundamentaldrwav_read_s32__alaw");
    build.define("modeldrwav_read_s32__alaw", "modelFundamentaldrwav_read_s32__alaw");
    build.define("drwav_read_s32__alawWidget", "Fundamentaldrwav_read_s32__alawWidget");
    build.define("drwav_read_s32__alaw", "Fundamentaldrwav_read_s32__alaw");
    build.define("modeldrwav_read_s32__alaw", "modelFundamentaldrwav_read_s32__alaw");
    build.define("drwav_read_s32__alawWidget", "Fundamentaldrwav_read_s32__alawWidget");
    build.define("drwav_read_s32__ieee", "Fundamentaldrwav_read_s32__ieee");
    build.define("modeldrwav_read_s32__ieee", "modelFundamentaldrwav_read_s32__ieee");
    build.define("drwav_read_s32__ieeeWidget", "Fundamentaldrwav_read_s32__ieeeWidget");
    build.define("drwav_read_s32__ieee", "Fundamentaldrwav_read_s32__ieee");
    build.define("modeldrwav_read_s32__ieee", "modelFundamentaldrwav_read_s32__ieee");
    build.define("drwav_read_s32__ieeeWidget", "Fundamentaldrwav_read_s32__ieeeWidget");
    build.define("drwav_read_s32__ima", "Fundamentaldrwav_read_s32__ima");
    build.define("modeldrwav_read_s32__ima", "modelFundamentaldrwav_read_s32__ima");
    build.define("drwav_read_s32__imaWidget", "Fundamentaldrwav_read_s32__imaWidget");
    build.define("drwav_read_s32__ima", "Fundamentaldrwav_read_s32__ima");
    build.define("modeldrwav_read_s32__ima", "modelFundamentaldrwav_read_s32__ima");
    build.define("drwav_read_s32__imaWidget", "Fundamentaldrwav_read_s32__imaWidget");
    build.define("drwav_read_s32__msadpcm", "Fundamentaldrwav_read_s32__msadpcm");
    build.define("modeldrwav_read_s32__msadpcm", "modelFundamentaldrwav_read_s32__msadpcm");
    build.define("drwav_read_s32__msadpcmWidget", "Fundamentaldrwav_read_s32__msadpcmWidget");
    build.define("drwav_read_s32__msadpcm", "Fundamentaldrwav_read_s32__msadpcm");
    build.define("modeldrwav_read_s32__msadpcm", "modelFundamentaldrwav_read_s32__msadpcm");
    build.define("drwav_read_s32__msadpcmWidget", "Fundamentaldrwav_read_s32__msadpcmWidget");
    build.define("drwav_read_s32__mulaw", "Fundamentaldrwav_read_s32__mulaw");
    build.define("modeldrwav_read_s32__mulaw", "modelFundamentaldrwav_read_s32__mulaw");
    build.define("drwav_read_s32__mulawWidget", "Fundamentaldrwav_read_s32__mulawWidget");
    build.define("drwav_read_s32__mulaw", "Fundamentaldrwav_read_s32__mulaw");
    build.define("modeldrwav_read_s32__mulaw", "modelFundamentaldrwav_read_s32__mulaw");
    build.define("drwav_read_s32__mulawWidget", "Fundamentaldrwav_read_s32__mulawWidget");
    build.define("drwav_read_s32__pcm", "Fundamentaldrwav_read_s32__pcm");
    build.define("modeldrwav_read_s32__pcm", "modelFundamentaldrwav_read_s32__pcm");
    build.define("drwav_read_s32__pcmWidget", "Fundamentaldrwav_read_s32__pcmWidget");
    build.define("drwav_read_s32__pcm", "Fundamentaldrwav_read_s32__pcm");
    build.define("modeldrwav_read_s32__pcm", "modelFundamentaldrwav_read_s32__pcm");
    build.define("drwav_read_s32__pcmWidget", "Fundamentaldrwav_read_s32__pcmWidget");
    build.define("drwav_riff_chunk_size_riff", "Fundamentaldrwav_riff_chunk_size_riff");
    build.define("modeldrwav_riff_chunk_size_riff", "modelFundamentaldrwav_riff_chunk_size_riff");
    build.define("drwav_riff_chunk_size_riffWidget", "Fundamentaldrwav_riff_chunk_size_riffWidget");
    build.define("drwav_riff_chunk_size_w64", "Fundamentaldrwav_riff_chunk_size_w64");
    build.define("modeldrwav_riff_chunk_size_w64", "modelFundamentaldrwav_riff_chunk_size_w64");
    build.define("drwav_riff_chunk_size_w64Widget", "Fundamentaldrwav_riff_chunk_size_w64Widget");
    build.define("drwav_s16_to_f32", "Fundamentaldrwav_s16_to_f32");
    build.define("modeldrwav_s16_to_f32", "modelFundamentaldrwav_s16_to_f32");
    build.define("drwav_s16_to_f32Widget", "Fundamentaldrwav_s16_to_f32Widget");
    build.define("drwav_s16_to_s32", "Fundamentaldrwav_s16_to_s32");
    build.define("modeldrwav_s16_to_s32", "modelFundamentaldrwav_s16_to_s32");
    build.define("drwav_s16_to_s32Widget", "Fundamentaldrwav_s16_to_s32Widget");
    build.define("drwav_s24_to_f32", "Fundamentaldrwav_s24_to_f32");
    build.define("modeldrwav_s24_to_f32", "modelFundamentaldrwav_s24_to_f32");
    build.define("drwav_s24_to_f32Widget", "Fundamentaldrwav_s24_to_f32Widget");
    build.define("drwav_s24_to_s16", "Fundamentaldrwav_s24_to_s16");
    build.define("modeldrwav_s24_to_s16", "modelFundamentaldrwav_s24_to_s16");
    build.define("drwav_s24_to_s16Widget", "Fundamentaldrwav_s24_to_s16Widget");
    build.define("drwav_s24_to_s16", "Fundamentaldrwav_s24_to_s16");
    build.define("modeldrwav_s24_to_s16", "modelFundamentaldrwav_s24_to_s16");
    build.define("drwav_s24_to_s16Widget", "Fundamentaldrwav_s24_to_s16Widget");
    build.define("drwav_s24_to_s32", "Fundamentaldrwav_s24_to_s32");
    build.define("modeldrwav_s24_to_s32", "modelFundamentaldrwav_s24_to_s32");
    build.define("drwav_s24_to_s32Widget", "Fundamentaldrwav_s24_to_s32Widget");
    build.define("drwav_s32_to_f32", "Fundamentaldrwav_s32_to_f32");
    build.define("modeldrwav_s32_to_f32", "modelFundamentaldrwav_s32_to_f32");
    build.define("drwav_s32_to_f32Widget", "Fundamentaldrwav_s32_to_f32Widget");
    build.define("drwav_s32_to_s16", "Fundamentaldrwav_s32_to_s16");
    build.define("modeldrwav_s32_to_s16", "modelFundamentaldrwav_s32_to_s16");
    build.define("drwav_s32_to_s16Widget", "Fundamentaldrwav_s32_to_s16Widget");
    build.define("drwav_s32_to_s16", "Fundamentaldrwav_s32_to_s16");
    build.define("modeldrwav_s32_to_s16", "modelFundamentaldrwav_s32_to_s16");
    build.define("drwav_s32_to_s16Widget", "Fundamentaldrwav_s32_to_s16Widget");
    build.define("drwav_seek_to_pcm_frame", "Fundamentaldrwav_seek_to_pcm_frame");
    build.define("modeldrwav_seek_to_pcm_frame", "modelFundamentaldrwav_seek_to_pcm_frame");
    build.define("drwav_seek_to_pcm_frameWidget", "Fundamentaldrwav_seek_to_pcm_frameWidget");
    build.define("drwav_seek_to_sample", "Fundamentaldrwav_seek_to_sample");
    build.define("modeldrwav_seek_to_sample", "modelFundamentaldrwav_seek_to_sample");
    build.define("drwav_seek_to_sampleWidget", "Fundamentaldrwav_seek_to_sampleWidget");
    build.define("drwav_seek_to_sample", "Fundamentaldrwav_seek_to_sample");
    build.define("modeldrwav_seek_to_sample", "modelFundamentaldrwav_seek_to_sample");
    build.define("drwav_seek_to_sampleWidget", "Fundamentaldrwav_seek_to_sampleWidget");
    build.define("drwav_smpl", "Fundamentaldrwav_smpl");
    build.define("modeldrwav_smpl", "modelFundamentaldrwav_smpl");
    build.define("drwav_smplWidget", "Fundamentaldrwav_smplWidget");
    build.define("drwav_smpl_loop", "Fundamentaldrwav_smpl_loop");
    build.define("modeldrwav_smpl_loop", "modelFundamentaldrwav_smpl_loop");
    build.define("drwav_smpl_loopWidget", "Fundamentaldrwav_smpl_loopWidget");
    build.define("drwav_take_ownership_of_metadata", "Fundamentaldrwav_take_ownership_of_metadata");
    build.define("modeldrwav_take_ownership_of_metadata", "modelFundamentaldrwav_take_ownership_of_metadata");
    build.define("drwav_take_ownership_of_metadataWidget", "Fundamentaldrwav_take_ownership_of_metadataWidget");
    build.define("drwav_target_write_size_bytes", "Fundamentaldrwav_target_write_size_bytes");
    build.define("modeldrwav_target_write_size_bytes", "modelFundamentaldrwav_target_write_size_bytes");
    build.define("drwav_target_write_size_bytesWidget", "Fundamentaldrwav_target_write_size_bytesWidget");
    build.define("drwav_u8_to_f32", "Fundamentaldrwav_u8_to_f32");
    build.define("modeldrwav_u8_to_f32", "modelFundamentaldrwav_u8_to_f32");
    build.define("drwav_u8_to_f32Widget", "Fundamentaldrwav_u8_to_f32Widget");
    build.define("drwav_u8_to_s16", "Fundamentaldrwav_u8_to_s16");
    build.define("modeldrwav_u8_to_s16", "modelFundamentaldrwav_u8_to_s16");
    build.define("drwav_u8_to_s16Widget", "Fundamentaldrwav_u8_to_s16Widget");
    build.define("drwav_u8_to_s16", "Fundamentaldrwav_u8_to_s16");
    build.define("modeldrwav_u8_to_s16", "modelFundamentaldrwav_u8_to_s16");
    build.define("drwav_u8_to_s16Widget", "Fundamentaldrwav_u8_to_s16Widget");
    build.define("drwav_u8_to_s32", "Fundamentaldrwav_u8_to_s32");
    build.define("modeldrwav_u8_to_s32", "modelFundamentaldrwav_u8_to_s32");
    build.define("drwav_u8_to_s32Widget", "Fundamentaldrwav_u8_to_s32Widget");
    build.define("drwav_uninit", "Fundamentaldrwav_uninit");
    build.define("modeldrwav_uninit", "modelFundamentaldrwav_uninit");
    build.define("drwav_uninitWidget", "Fundamentaldrwav_uninitWidget");
    build.define("drwav_version", "Fundamentaldrwav_version");
    build.define("modeldrwav_version", "modelFundamentaldrwav_version");
    build.define("drwav_versionWidget", "Fundamentaldrwav_versionWidget");
    build.define("drwav_version_string", "Fundamentaldrwav_version_string");
    build.define("modeldrwav_version_string", "modelFundamentaldrwav_version_string");
    build.define("drwav_version_stringWidget", "Fundamentaldrwav_version_stringWidget");
    build.define("drwav_write", "Fundamentaldrwav_write");
    build.define("modeldrwav_write", "modelFundamentaldrwav_write");
    build.define("drwav_writeWidget", "Fundamentaldrwav_writeWidget");
    build.define("drwav_write", "Fundamentaldrwav_write");
    build.define("modeldrwav_write", "modelFundamentaldrwav_write");
    build.define("drwav_writeWidget", "Fundamentaldrwav_writeWidget");
    build.define("drwav_write_pcm_frames", "Fundamentaldrwav_write_pcm_frames");
    build.define("modeldrwav_write_pcm_frames", "modelFundamentaldrwav_write_pcm_frames");
    build.define("drwav_write_pcm_framesWidget", "Fundamentaldrwav_write_pcm_framesWidget");
    build.define("drwav_write_pcm_frames_be", "Fundamentaldrwav_write_pcm_frames_be");
    build.define("modeldrwav_write_pcm_frames_be", "modelFundamentaldrwav_write_pcm_frames_be");
    build.define("drwav_write_pcm_frames_beWidget", "Fundamentaldrwav_write_pcm_frames_beWidget");
    build.define("drwav_write_pcm_frames_le", "Fundamentaldrwav_write_pcm_frames_le");
    build.define("modeldrwav_write_pcm_frames_le", "modelFundamentaldrwav_write_pcm_frames_le");
    build.define("drwav_write_pcm_frames_leWidget", "Fundamentaldrwav_write_pcm_frames_leWidget");
    build.define("drwav_write_raw", "Fundamentaldrwav_write_raw");
    build.define("modeldrwav_write_raw", "modelFundamentaldrwav_write_raw");
    build.define("drwav_write_rawWidget", "Fundamentaldrwav_write_rawWidget");

    // Filter-out list
    let _filter_out: Vec<String> = vec![
        "Fundamental/src/plugin.cpp".to_string(),
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
    collect_sources(&plugins_dir.join("Fundamental/src"), &_filter_out, &plugins_dir, &mut build, 0);
    build.file(plugins_dir.join("Fundamental/src/dr_wav.c"));

    // Init wrapper (renames init() only for the plugin registration file)
    build.file(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("register.cpp"));

    println!("cargo:rerun-if-changed=register.cpp");
    build.compile("cardinal_plugin_fundamental");
}
