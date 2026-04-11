use std::path::PathBuf;

fn main() {
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // cardinal-rs/
        .parent().unwrap()  // Cardinal/
        .to_path_buf();

    let plugins_dir = cardinal_root.join("plugins");
    let plugin_dir = plugins_dir.join("ValleyAudio");

    if !plugin_dir.exists() {
        eprintln!("Plugin ValleyAudio not found (submodule not initialized?), skipping");
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
    build.define("pluginInstance", "pluginInstance__ValleyAudio");
    build.define("init", "init__ValleyAudio");
    build.define("drwav", "ValleyAudiodrwav");
    build.define("modeldrwav", "modelValleyAudiodrwav");
    build.define("drwavWidget", "ValleyAudiodrwavWidget");
    build.define("drwav__on_read", "ValleyAudiodrwav__on_read");
    build.define("modeldrwav__on_read", "modelValleyAudiodrwav__on_read");
    build.define("drwav__on_readWidget", "ValleyAudiodrwav__on_readWidget");
    build.define("drwav__on_seek", "ValleyAudiodrwav__on_seek");
    build.define("modeldrwav__on_seek", "modelValleyAudiodrwav__on_seek");
    build.define("drwav__on_seekWidget", "ValleyAudiodrwav__on_seekWidget");
    build.define("drwav__read_and_close_f32", "ValleyAudiodrwav__read_and_close_f32");
    build.define("modeldrwav__read_and_close_f32", "modelValleyAudiodrwav__read_and_close_f32");
    build.define("drwav__read_and_close_f32Widget", "ValleyAudiodrwav__read_and_close_f32Widget");
    build.define("drwav__read_and_close_s16", "ValleyAudiodrwav__read_and_close_s16");
    build.define("modeldrwav__read_and_close_s16", "modelValleyAudiodrwav__read_and_close_s16");
    build.define("drwav__read_and_close_s16Widget", "ValleyAudiodrwav__read_and_close_s16Widget");
    build.define("drwav__read_and_close_s32", "ValleyAudiodrwav__read_and_close_s32");
    build.define("modeldrwav__read_and_close_s32", "modelValleyAudiodrwav__read_and_close_s32");
    build.define("drwav__read_and_close_s32Widget", "ValleyAudiodrwav__read_and_close_s32Widget");
    build.define("drwav_alaw_to_f32", "ValleyAudiodrwav_alaw_to_f32");
    build.define("modeldrwav_alaw_to_f32", "modelValleyAudiodrwav_alaw_to_f32");
    build.define("drwav_alaw_to_f32Widget", "ValleyAudiodrwav_alaw_to_f32Widget");
    build.define("drwav_alaw_to_s16", "ValleyAudiodrwav_alaw_to_s16");
    build.define("modeldrwav_alaw_to_s16", "modelValleyAudiodrwav_alaw_to_s16");
    build.define("drwav_alaw_to_s16Widget", "ValleyAudiodrwav_alaw_to_s16Widget");
    build.define("drwav_alaw_to_s16", "ValleyAudiodrwav_alaw_to_s16");
    build.define("modeldrwav_alaw_to_s16", "modelValleyAudiodrwav_alaw_to_s16");
    build.define("drwav_alaw_to_s16Widget", "ValleyAudiodrwav_alaw_to_s16Widget");
    build.define("drwav_alaw_to_s32", "ValleyAudiodrwav_alaw_to_s32");
    build.define("modeldrwav_alaw_to_s32", "modelValleyAudiodrwav_alaw_to_s32");
    build.define("drwav_alaw_to_s32Widget", "ValleyAudiodrwav_alaw_to_s32Widget");
    build.define("drwav_bytes_to_f32", "ValleyAudiodrwav_bytes_to_f32");
    build.define("modeldrwav_bytes_to_f32", "modelValleyAudiodrwav_bytes_to_f32");
    build.define("drwav_bytes_to_f32Widget", "ValleyAudiodrwav_bytes_to_f32Widget");
    build.define("drwav_bytes_to_s16", "ValleyAudiodrwav_bytes_to_s16");
    build.define("modeldrwav_bytes_to_s16", "modelValleyAudiodrwav_bytes_to_s16");
    build.define("drwav_bytes_to_s16Widget", "ValleyAudiodrwav_bytes_to_s16Widget");
    build.define("drwav_bytes_to_s32", "ValleyAudiodrwav_bytes_to_s32");
    build.define("modeldrwav_bytes_to_s32", "modelValleyAudiodrwav_bytes_to_s32");
    build.define("drwav_bytes_to_s32Widget", "ValleyAudiodrwav_bytes_to_s32Widget");
    build.define("drwav_bytes_to_s64", "ValleyAudiodrwav_bytes_to_s64");
    build.define("modeldrwav_bytes_to_s64", "modelValleyAudiodrwav_bytes_to_s64");
    build.define("drwav_bytes_to_s64Widget", "ValleyAudiodrwav_bytes_to_s64Widget");
    build.define("drwav_bytes_to_u16", "ValleyAudiodrwav_bytes_to_u16");
    build.define("modeldrwav_bytes_to_u16", "modelValleyAudiodrwav_bytes_to_u16");
    build.define("drwav_bytes_to_u16Widget", "ValleyAudiodrwav_bytes_to_u16Widget");
    build.define("drwav_bytes_to_u32", "ValleyAudiodrwav_bytes_to_u32");
    build.define("modeldrwav_bytes_to_u32", "modelValleyAudiodrwav_bytes_to_u32");
    build.define("drwav_bytes_to_u32Widget", "ValleyAudiodrwav_bytes_to_u32Widget");
    build.define("drwav_bytes_to_u64", "ValleyAudiodrwav_bytes_to_u64");
    build.define("modeldrwav_bytes_to_u64", "modelValleyAudiodrwav_bytes_to_u64");
    build.define("drwav_bytes_to_u64Widget", "ValleyAudiodrwav_bytes_to_u64Widget");
    build.define("drwav_close", "ValleyAudiodrwav_close");
    build.define("modeldrwav_close", "modelValleyAudiodrwav_close");
    build.define("drwav_closeWidget", "ValleyAudiodrwav_closeWidget");
    build.define("drwav_close", "ValleyAudiodrwav_close");
    build.define("modeldrwav_close", "modelValleyAudiodrwav_close");
    build.define("drwav_closeWidget", "ValleyAudiodrwav_closeWidget");
    build.define("drwav_container", "ValleyAudiodrwav_container");
    build.define("modeldrwav_container", "modelValleyAudiodrwav_container");
    build.define("drwav_containerWidget", "ValleyAudiodrwav_containerWidget");
    build.define("drwav_data_chunk_size_riff", "ValleyAudiodrwav_data_chunk_size_riff");
    build.define("modeldrwav_data_chunk_size_riff", "modelValleyAudiodrwav_data_chunk_size_riff");
    build.define("drwav_data_chunk_size_riffWidget", "ValleyAudiodrwav_data_chunk_size_riffWidget");
    build.define("drwav_data_chunk_size_w64", "ValleyAudiodrwav_data_chunk_size_w64");
    build.define("modeldrwav_data_chunk_size_w64", "modelValleyAudiodrwav_data_chunk_size_w64");
    build.define("drwav_data_chunk_size_w64Widget", "ValleyAudiodrwav_data_chunk_size_w64Widget");
    build.define("drwav_data_format", "ValleyAudiodrwav_data_format");
    build.define("modeldrwav_data_format", "modelValleyAudiodrwav_data_format");
    build.define("drwav_data_formatWidget", "ValleyAudiodrwav_data_formatWidget");
    build.define("drwav_f32_to_s16", "ValleyAudiodrwav_f32_to_s16");
    build.define("modeldrwav_f32_to_s16", "modelValleyAudiodrwav_f32_to_s16");
    build.define("drwav_f32_to_s16Widget", "ValleyAudiodrwav_f32_to_s16Widget");
    build.define("drwav_f32_to_s32", "ValleyAudiodrwav_f32_to_s32");
    build.define("modeldrwav_f32_to_s32", "modelValleyAudiodrwav_f32_to_s32");
    build.define("drwav_f32_to_s32Widget", "ValleyAudiodrwav_f32_to_s32Widget");
    build.define("drwav_f64_to_f32", "ValleyAudiodrwav_f64_to_f32");
    build.define("modeldrwav_f64_to_f32", "modelValleyAudiodrwav_f64_to_f32");
    build.define("drwav_f64_to_f32Widget", "ValleyAudiodrwav_f64_to_f32Widget");
    build.define("drwav_f64_to_s16", "ValleyAudiodrwav_f64_to_s16");
    build.define("modeldrwav_f64_to_s16", "modelValleyAudiodrwav_f64_to_s16");
    build.define("drwav_f64_to_s16Widget", "ValleyAudiodrwav_f64_to_s16Widget");
    build.define("drwav_f64_to_s16", "ValleyAudiodrwav_f64_to_s16");
    build.define("modeldrwav_f64_to_s16", "modelValleyAudiodrwav_f64_to_s16");
    build.define("drwav_f64_to_s16Widget", "ValleyAudiodrwav_f64_to_s16Widget");
    build.define("drwav_f64_to_s32", "ValleyAudiodrwav_f64_to_s32");
    build.define("modeldrwav_f64_to_s32", "modelValleyAudiodrwav_f64_to_s32");
    build.define("drwav_f64_to_s32Widget", "ValleyAudiodrwav_f64_to_s32Widget");
    build.define("drwav_fmt_get_format", "ValleyAudiodrwav_fmt_get_format");
    build.define("modeldrwav_fmt_get_format", "modelValleyAudiodrwav_fmt_get_format");
    build.define("drwav_fmt_get_formatWidget", "ValleyAudiodrwav_fmt_get_formatWidget");
    build.define("drwav_fopen", "ValleyAudiodrwav_fopen");
    build.define("modeldrwav_fopen", "modelValleyAudiodrwav_fopen");
    build.define("drwav_fopenWidget", "ValleyAudiodrwav_fopenWidget");
    build.define("drwav_fourcc_equal", "ValleyAudiodrwav_fourcc_equal");
    build.define("modeldrwav_fourcc_equal", "modelValleyAudiodrwav_fourcc_equal");
    build.define("drwav_fourcc_equalWidget", "ValleyAudiodrwav_fourcc_equalWidget");
    build.define("drwav_free", "ValleyAudiodrwav_free");
    build.define("modeldrwav_free", "modelValleyAudiodrwav_free");
    build.define("drwav_freeWidget", "ValleyAudiodrwav_freeWidget");
    build.define("drwav_get_cursor_in_pcm_frames", "ValleyAudiodrwav_get_cursor_in_pcm_frames");
    build.define("modeldrwav_get_cursor_in_pcm_frames", "modelValleyAudiodrwav_get_cursor_in_pcm_frames");
    build.define("drwav_get_cursor_in_pcm_framesWidget", "ValleyAudiodrwav_get_cursor_in_pcm_framesWidget");
    build.define("drwav_get_length_in_pcm_frames", "ValleyAudiodrwav_get_length_in_pcm_frames");
    build.define("modeldrwav_get_length_in_pcm_frames", "modelValleyAudiodrwav_get_length_in_pcm_frames");
    build.define("drwav_get_length_in_pcm_framesWidget", "ValleyAudiodrwav_get_length_in_pcm_framesWidget");
    build.define("drwav_guid_equal", "ValleyAudiodrwav_guid_equal");
    build.define("modeldrwav_guid_equal", "modelValleyAudiodrwav_guid_equal");
    build.define("drwav_guid_equalWidget", "ValleyAudiodrwav_guid_equalWidget");
    build.define("drwav_init", "ValleyAudiodrwav_init");
    build.define("modeldrwav_init", "modelValleyAudiodrwav_init");
    build.define("drwav_initWidget", "ValleyAudiodrwav_initWidget");
    build.define("drwav_init_ex", "ValleyAudiodrwav_init_ex");
    build.define("modeldrwav_init_ex", "modelValleyAudiodrwav_init_ex");
    build.define("drwav_init_exWidget", "ValleyAudiodrwav_init_exWidget");
    build.define("drwav_init_file", "ValleyAudiodrwav_init_file");
    build.define("modeldrwav_init_file", "modelValleyAudiodrwav_init_file");
    build.define("drwav_init_fileWidget", "ValleyAudiodrwav_init_fileWidget");
    build.define("drwav_init_file_ex", "ValleyAudiodrwav_init_file_ex");
    build.define("modeldrwav_init_file_ex", "modelValleyAudiodrwav_init_file_ex");
    build.define("drwav_init_file_exWidget", "ValleyAudiodrwav_init_file_exWidget");
    build.define("drwav_init_file_ex_w", "ValleyAudiodrwav_init_file_ex_w");
    build.define("modeldrwav_init_file_ex_w", "modelValleyAudiodrwav_init_file_ex_w");
    build.define("drwav_init_file_ex_wWidget", "ValleyAudiodrwav_init_file_ex_wWidget");
    build.define("drwav_init_file_w", "ValleyAudiodrwav_init_file_w");
    build.define("modeldrwav_init_file_w", "modelValleyAudiodrwav_init_file_w");
    build.define("drwav_init_file_wWidget", "ValleyAudiodrwav_init_file_wWidget");
    build.define("drwav_init_file_with_metadata", "ValleyAudiodrwav_init_file_with_metadata");
    build.define("modeldrwav_init_file_with_metadata", "modelValleyAudiodrwav_init_file_with_metadata");
    build.define("drwav_init_file_with_metadataWidget", "ValleyAudiodrwav_init_file_with_metadataWidget");
    build.define("drwav_init_file_with_metadata_w", "ValleyAudiodrwav_init_file_with_metadata_w");
    build.define("modeldrwav_init_file_with_metadata_w", "modelValleyAudiodrwav_init_file_with_metadata_w");
    build.define("drwav_init_file_with_metadata_wWidget", "ValleyAudiodrwav_init_file_with_metadata_wWidget");
    build.define("drwav_init_file_write", "ValleyAudiodrwav_init_file_write");
    build.define("modeldrwav_init_file_write", "modelValleyAudiodrwav_init_file_write");
    build.define("drwav_init_file_writeWidget", "ValleyAudiodrwav_init_file_writeWidget");
    build.define("drwav_init_file_write", "ValleyAudiodrwav_init_file_write");
    build.define("modeldrwav_init_file_write", "modelValleyAudiodrwav_init_file_write");
    build.define("drwav_init_file_writeWidget", "ValleyAudiodrwav_init_file_writeWidget");
    build.define("drwav_init_file_write__internal", "ValleyAudiodrwav_init_file_write__internal");
    build.define("modeldrwav_init_file_write__internal", "modelValleyAudiodrwav_init_file_write__internal");
    build.define("drwav_init_file_write__internalWidget", "ValleyAudiodrwav_init_file_write__internalWidget");
    build.define("drwav_init_file_write__internal", "ValleyAudiodrwav_init_file_write__internal");
    build.define("modeldrwav_init_file_write__internal", "modelValleyAudiodrwav_init_file_write__internal");
    build.define("drwav_init_file_write__internalWidget", "ValleyAudiodrwav_init_file_write__internalWidget");
    build.define("drwav_init_file_write_sequential", "ValleyAudiodrwav_init_file_write_sequential");
    build.define("modeldrwav_init_file_write_sequential", "modelValleyAudiodrwav_init_file_write_sequential");
    build.define("drwav_init_file_write_sequentialWidget", "ValleyAudiodrwav_init_file_write_sequentialWidget");
    build.define("drwav_init_file_write_sequential", "ValleyAudiodrwav_init_file_write_sequential");
    build.define("modeldrwav_init_file_write_sequential", "modelValleyAudiodrwav_init_file_write_sequential");
    build.define("drwav_init_file_write_sequentialWidget", "ValleyAudiodrwav_init_file_write_sequentialWidget");
    build.define("drwav_init_file_write_sequential_pcm_frames", "ValleyAudiodrwav_init_file_write_sequential_pcm_frames");
    build.define("modeldrwav_init_file_write_sequential_pcm_frames", "modelValleyAudiodrwav_init_file_write_sequential_pcm_frames");
    build.define("drwav_init_file_write_sequential_pcm_framesWidget", "ValleyAudiodrwav_init_file_write_sequential_pcm_framesWidget");
    build.define("drwav_init_file_write_sequential_pcm_frames_w", "ValleyAudiodrwav_init_file_write_sequential_pcm_frames_w");
    build.define("modeldrwav_init_file_write_sequential_pcm_frames_w", "modelValleyAudiodrwav_init_file_write_sequential_pcm_frames_w");
    build.define("drwav_init_file_write_sequential_pcm_frames_wWidget", "ValleyAudiodrwav_init_file_write_sequential_pcm_frames_wWidget");
    build.define("drwav_init_file_write_sequential_w", "ValleyAudiodrwav_init_file_write_sequential_w");
    build.define("modeldrwav_init_file_write_sequential_w", "modelValleyAudiodrwav_init_file_write_sequential_w");
    build.define("drwav_init_file_write_sequential_wWidget", "ValleyAudiodrwav_init_file_write_sequential_wWidget");
    build.define("drwav_init_file_write_w", "ValleyAudiodrwav_init_file_write_w");
    build.define("modeldrwav_init_file_write_w", "modelValleyAudiodrwav_init_file_write_w");
    build.define("drwav_init_file_write_wWidget", "ValleyAudiodrwav_init_file_write_wWidget");
    build.define("drwav_init_memory", "ValleyAudiodrwav_init_memory");
    build.define("modeldrwav_init_memory", "modelValleyAudiodrwav_init_memory");
    build.define("drwav_init_memoryWidget", "ValleyAudiodrwav_init_memoryWidget");
    build.define("drwav_init_memory_ex", "ValleyAudiodrwav_init_memory_ex");
    build.define("modeldrwav_init_memory_ex", "modelValleyAudiodrwav_init_memory_ex");
    build.define("drwav_init_memory_exWidget", "ValleyAudiodrwav_init_memory_exWidget");
    build.define("drwav_init_memory_with_metadata", "ValleyAudiodrwav_init_memory_with_metadata");
    build.define("modeldrwav_init_memory_with_metadata", "modelValleyAudiodrwav_init_memory_with_metadata");
    build.define("drwav_init_memory_with_metadataWidget", "ValleyAudiodrwav_init_memory_with_metadataWidget");
    build.define("drwav_init_memory_write", "ValleyAudiodrwav_init_memory_write");
    build.define("modeldrwav_init_memory_write", "modelValleyAudiodrwav_init_memory_write");
    build.define("drwav_init_memory_writeWidget", "ValleyAudiodrwav_init_memory_writeWidget");
    build.define("drwav_init_memory_write", "ValleyAudiodrwav_init_memory_write");
    build.define("modeldrwav_init_memory_write", "modelValleyAudiodrwav_init_memory_write");
    build.define("drwav_init_memory_writeWidget", "ValleyAudiodrwav_init_memory_writeWidget");
    build.define("drwav_init_memory_write__internal", "ValleyAudiodrwav_init_memory_write__internal");
    build.define("modeldrwav_init_memory_write__internal", "modelValleyAudiodrwav_init_memory_write__internal");
    build.define("drwav_init_memory_write__internalWidget", "ValleyAudiodrwav_init_memory_write__internalWidget");
    build.define("drwav_init_memory_write__internal", "ValleyAudiodrwav_init_memory_write__internal");
    build.define("modeldrwav_init_memory_write__internal", "modelValleyAudiodrwav_init_memory_write__internal");
    build.define("drwav_init_memory_write__internalWidget", "ValleyAudiodrwav_init_memory_write__internalWidget");
    build.define("drwav_init_memory_write_sequential", "ValleyAudiodrwav_init_memory_write_sequential");
    build.define("modeldrwav_init_memory_write_sequential", "modelValleyAudiodrwav_init_memory_write_sequential");
    build.define("drwav_init_memory_write_sequentialWidget", "ValleyAudiodrwav_init_memory_write_sequentialWidget");
    build.define("drwav_init_memory_write_sequential_pcm_frames", "ValleyAudiodrwav_init_memory_write_sequential_pcm_frames");
    build.define("modeldrwav_init_memory_write_sequential_pcm_frames", "modelValleyAudiodrwav_init_memory_write_sequential_pcm_frames");
    build.define("drwav_init_memory_write_sequential_pcm_framesWidget", "ValleyAudiodrwav_init_memory_write_sequential_pcm_framesWidget");
    build.define("drwav_init_with_metadata", "ValleyAudiodrwav_init_with_metadata");
    build.define("modeldrwav_init_with_metadata", "modelValleyAudiodrwav_init_with_metadata");
    build.define("drwav_init_with_metadataWidget", "ValleyAudiodrwav_init_with_metadataWidget");
    build.define("drwav_init_write", "ValleyAudiodrwav_init_write");
    build.define("modeldrwav_init_write", "modelValleyAudiodrwav_init_write");
    build.define("drwav_init_writeWidget", "ValleyAudiodrwav_init_writeWidget");
    build.define("drwav_init_write", "ValleyAudiodrwav_init_write");
    build.define("modeldrwav_init_write", "modelValleyAudiodrwav_init_write");
    build.define("drwav_init_writeWidget", "ValleyAudiodrwav_init_writeWidget");
    build.define("drwav_init_write__internal", "ValleyAudiodrwav_init_write__internal");
    build.define("modeldrwav_init_write__internal", "modelValleyAudiodrwav_init_write__internal");
    build.define("drwav_init_write__internalWidget", "ValleyAudiodrwav_init_write__internalWidget");
    build.define("drwav_init_write_sequential", "ValleyAudiodrwav_init_write_sequential");
    build.define("modeldrwav_init_write_sequential", "modelValleyAudiodrwav_init_write_sequential");
    build.define("drwav_init_write_sequentialWidget", "ValleyAudiodrwav_init_write_sequentialWidget");
    build.define("drwav_init_write_sequential_pcm_frames", "ValleyAudiodrwav_init_write_sequential_pcm_frames");
    build.define("modeldrwav_init_write_sequential_pcm_frames", "modelValleyAudiodrwav_init_write_sequential_pcm_frames");
    build.define("drwav_init_write_sequential_pcm_framesWidget", "ValleyAudiodrwav_init_write_sequential_pcm_framesWidget");
    build.define("drwav_init_write_with_metadata", "ValleyAudiodrwav_init_write_with_metadata");
    build.define("modeldrwav_init_write_with_metadata", "modelValleyAudiodrwav_init_write_with_metadata");
    build.define("drwav_init_write_with_metadataWidget", "ValleyAudiodrwav_init_write_with_metadataWidget");
    build.define("drwav_metadata", "ValleyAudiodrwav_metadata");
    build.define("modeldrwav_metadata", "modelValleyAudiodrwav_metadata");
    build.define("drwav_metadataWidget", "ValleyAudiodrwav_metadataWidget");
    build.define("drwav__metadata_parser", "ValleyAudiodrwav__metadata_parser");
    build.define("modeldrwav__metadata_parser", "modelValleyAudiodrwav__metadata_parser");
    build.define("drwav__metadata_parserWidget", "ValleyAudiodrwav__metadata_parserWidget");
    build.define("drwav_mulaw_to_f32", "ValleyAudiodrwav_mulaw_to_f32");
    build.define("modeldrwav_mulaw_to_f32", "modelValleyAudiodrwav_mulaw_to_f32");
    build.define("drwav_mulaw_to_f32Widget", "ValleyAudiodrwav_mulaw_to_f32Widget");
    build.define("drwav_mulaw_to_s16", "ValleyAudiodrwav_mulaw_to_s16");
    build.define("modeldrwav_mulaw_to_s16", "modelValleyAudiodrwav_mulaw_to_s16");
    build.define("drwav_mulaw_to_s16Widget", "ValleyAudiodrwav_mulaw_to_s16Widget");
    build.define("drwav_mulaw_to_s16", "ValleyAudiodrwav_mulaw_to_s16");
    build.define("modeldrwav_mulaw_to_s16", "modelValleyAudiodrwav_mulaw_to_s16");
    build.define("drwav_mulaw_to_s16Widget", "ValleyAudiodrwav_mulaw_to_s16Widget");
    build.define("drwav_mulaw_to_s32", "ValleyAudiodrwav_mulaw_to_s32");
    build.define("modeldrwav_mulaw_to_s32", "modelValleyAudiodrwav_mulaw_to_s32");
    build.define("drwav_mulaw_to_s32Widget", "ValleyAudiodrwav_mulaw_to_s32Widget");
    build.define("drwav_open", "ValleyAudiodrwav_open");
    build.define("modeldrwav_open", "modelValleyAudiodrwav_open");
    build.define("drwav_openWidget", "ValleyAudiodrwav_openWidget");
    build.define("drwav_open_and_read_f32", "ValleyAudiodrwav_open_and_read_f32");
    build.define("modeldrwav_open_and_read_f32", "modelValleyAudiodrwav_open_and_read_f32");
    build.define("drwav_open_and_read_f32Widget", "ValleyAudiodrwav_open_and_read_f32Widget");
    build.define("drwav_open_and_read_file_f32", "ValleyAudiodrwav_open_and_read_file_f32");
    build.define("modeldrwav_open_and_read_file_f32", "modelValleyAudiodrwav_open_and_read_file_f32");
    build.define("drwav_open_and_read_file_f32Widget", "ValleyAudiodrwav_open_and_read_file_f32Widget");
    build.define("drwav_open_and_read_file_s16", "ValleyAudiodrwav_open_and_read_file_s16");
    build.define("modeldrwav_open_and_read_file_s16", "modelValleyAudiodrwav_open_and_read_file_s16");
    build.define("drwav_open_and_read_file_s16Widget", "ValleyAudiodrwav_open_and_read_file_s16Widget");
    build.define("drwav_open_and_read_file_s32", "ValleyAudiodrwav_open_and_read_file_s32");
    build.define("modeldrwav_open_and_read_file_s32", "modelValleyAudiodrwav_open_and_read_file_s32");
    build.define("drwav_open_and_read_file_s32Widget", "ValleyAudiodrwav_open_and_read_file_s32Widget");
    build.define("drwav_open_and_read_memory_f32", "ValleyAudiodrwav_open_and_read_memory_f32");
    build.define("modeldrwav_open_and_read_memory_f32", "modelValleyAudiodrwav_open_and_read_memory_f32");
    build.define("drwav_open_and_read_memory_f32Widget", "ValleyAudiodrwav_open_and_read_memory_f32Widget");
    build.define("drwav_open_and_read_memory_s16", "ValleyAudiodrwav_open_and_read_memory_s16");
    build.define("modeldrwav_open_and_read_memory_s16", "modelValleyAudiodrwav_open_and_read_memory_s16");
    build.define("drwav_open_and_read_memory_s16Widget", "ValleyAudiodrwav_open_and_read_memory_s16Widget");
    build.define("drwav_open_and_read_memory_s32", "ValleyAudiodrwav_open_and_read_memory_s32");
    build.define("modeldrwav_open_and_read_memory_s32", "modelValleyAudiodrwav_open_and_read_memory_s32");
    build.define("drwav_open_and_read_memory_s32Widget", "ValleyAudiodrwav_open_and_read_memory_s32Widget");
    build.define("drwav_open_and_read_pcm_frames_f32", "ValleyAudiodrwav_open_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_and_read_pcm_frames_f32", "modelValleyAudiodrwav_open_and_read_pcm_frames_f32");
    build.define("drwav_open_and_read_pcm_frames_f32Widget", "ValleyAudiodrwav_open_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_and_read_pcm_frames_s16", "ValleyAudiodrwav_open_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_and_read_pcm_frames_s16", "modelValleyAudiodrwav_open_and_read_pcm_frames_s16");
    build.define("drwav_open_and_read_pcm_frames_s16Widget", "ValleyAudiodrwav_open_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_and_read_pcm_frames_s32", "ValleyAudiodrwav_open_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_and_read_pcm_frames_s32", "modelValleyAudiodrwav_open_and_read_pcm_frames_s32");
    build.define("drwav_open_and_read_pcm_frames_s32Widget", "ValleyAudiodrwav_open_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_and_read_s16", "ValleyAudiodrwav_open_and_read_s16");
    build.define("modeldrwav_open_and_read_s16", "modelValleyAudiodrwav_open_and_read_s16");
    build.define("drwav_open_and_read_s16Widget", "ValleyAudiodrwav_open_and_read_s16Widget");
    build.define("drwav_open_and_read_s32", "ValleyAudiodrwav_open_and_read_s32");
    build.define("modeldrwav_open_and_read_s32", "modelValleyAudiodrwav_open_and_read_s32");
    build.define("drwav_open_and_read_s32Widget", "ValleyAudiodrwav_open_and_read_s32Widget");
    build.define("drwav_open_ex", "ValleyAudiodrwav_open_ex");
    build.define("modeldrwav_open_ex", "modelValleyAudiodrwav_open_ex");
    build.define("drwav_open_exWidget", "ValleyAudiodrwav_open_exWidget");
    build.define("drwav_open_file", "ValleyAudiodrwav_open_file");
    build.define("modeldrwav_open_file", "modelValleyAudiodrwav_open_file");
    build.define("drwav_open_fileWidget", "ValleyAudiodrwav_open_fileWidget");
    build.define("drwav_open_file_and_read_f32", "ValleyAudiodrwav_open_file_and_read_f32");
    build.define("modeldrwav_open_file_and_read_f32", "modelValleyAudiodrwav_open_file_and_read_f32");
    build.define("drwav_open_file_and_read_f32Widget", "ValleyAudiodrwav_open_file_and_read_f32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_f32", "ValleyAudiodrwav_open_file_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_file_and_read_pcm_frames_f32", "modelValleyAudiodrwav_open_file_and_read_pcm_frames_f32");
    build.define("drwav_open_file_and_read_pcm_frames_f32Widget", "ValleyAudiodrwav_open_file_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_f32_w", "ValleyAudiodrwav_open_file_and_read_pcm_frames_f32_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_f32_w", "modelValleyAudiodrwav_open_file_and_read_pcm_frames_f32_w");
    build.define("drwav_open_file_and_read_pcm_frames_f32_wWidget", "ValleyAudiodrwav_open_file_and_read_pcm_frames_f32_wWidget");
    build.define("drwav_open_file_and_read_pcm_frames_s16", "ValleyAudiodrwav_open_file_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s16", "modelValleyAudiodrwav_open_file_and_read_pcm_frames_s16");
    build.define("drwav_open_file_and_read_pcm_frames_s16Widget", "ValleyAudiodrwav_open_file_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_file_and_read_pcm_frames_s16_w", "ValleyAudiodrwav_open_file_and_read_pcm_frames_s16_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s16_w", "modelValleyAudiodrwav_open_file_and_read_pcm_frames_s16_w");
    build.define("drwav_open_file_and_read_pcm_frames_s16_wWidget", "ValleyAudiodrwav_open_file_and_read_pcm_frames_s16_wWidget");
    build.define("drwav_open_file_and_read_pcm_frames_s32", "ValleyAudiodrwav_open_file_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s32", "modelValleyAudiodrwav_open_file_and_read_pcm_frames_s32");
    build.define("drwav_open_file_and_read_pcm_frames_s32Widget", "ValleyAudiodrwav_open_file_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_s32_w", "ValleyAudiodrwav_open_file_and_read_pcm_frames_s32_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s32_w", "modelValleyAudiodrwav_open_file_and_read_pcm_frames_s32_w");
    build.define("drwav_open_file_and_read_pcm_frames_s32_wWidget", "ValleyAudiodrwav_open_file_and_read_pcm_frames_s32_wWidget");
    build.define("drwav_open_file_and_read_s16", "ValleyAudiodrwav_open_file_and_read_s16");
    build.define("modeldrwav_open_file_and_read_s16", "modelValleyAudiodrwav_open_file_and_read_s16");
    build.define("drwav_open_file_and_read_s16Widget", "ValleyAudiodrwav_open_file_and_read_s16Widget");
    build.define("drwav_open_file_and_read_s32", "ValleyAudiodrwav_open_file_and_read_s32");
    build.define("modeldrwav_open_file_and_read_s32", "modelValleyAudiodrwav_open_file_and_read_s32");
    build.define("drwav_open_file_and_read_s32Widget", "ValleyAudiodrwav_open_file_and_read_s32Widget");
    build.define("drwav_open_file_ex", "ValleyAudiodrwav_open_file_ex");
    build.define("modeldrwav_open_file_ex", "modelValleyAudiodrwav_open_file_ex");
    build.define("drwav_open_file_exWidget", "ValleyAudiodrwav_open_file_exWidget");
    build.define("drwav_open_file_write", "ValleyAudiodrwav_open_file_write");
    build.define("modeldrwav_open_file_write", "modelValleyAudiodrwav_open_file_write");
    build.define("drwav_open_file_writeWidget", "ValleyAudiodrwav_open_file_writeWidget");
    build.define("drwav_open_file_write", "ValleyAudiodrwav_open_file_write");
    build.define("modeldrwav_open_file_write", "modelValleyAudiodrwav_open_file_write");
    build.define("drwav_open_file_writeWidget", "ValleyAudiodrwav_open_file_writeWidget");
    build.define("drwav_open_file_write__internal", "ValleyAudiodrwav_open_file_write__internal");
    build.define("modeldrwav_open_file_write__internal", "modelValleyAudiodrwav_open_file_write__internal");
    build.define("drwav_open_file_write__internalWidget", "ValleyAudiodrwav_open_file_write__internalWidget");
    build.define("drwav_open_file_write__internal", "ValleyAudiodrwav_open_file_write__internal");
    build.define("modeldrwav_open_file_write__internal", "modelValleyAudiodrwav_open_file_write__internal");
    build.define("drwav_open_file_write__internalWidget", "ValleyAudiodrwav_open_file_write__internalWidget");
    build.define("drwav_open_file_write_sequential", "ValleyAudiodrwav_open_file_write_sequential");
    build.define("modeldrwav_open_file_write_sequential", "modelValleyAudiodrwav_open_file_write_sequential");
    build.define("drwav_open_file_write_sequentialWidget", "ValleyAudiodrwav_open_file_write_sequentialWidget");
    build.define("drwav_open_file_write_sequential", "ValleyAudiodrwav_open_file_write_sequential");
    build.define("modeldrwav_open_file_write_sequential", "modelValleyAudiodrwav_open_file_write_sequential");
    build.define("drwav_open_file_write_sequentialWidget", "ValleyAudiodrwav_open_file_write_sequentialWidget");
    build.define("drwav_open_memory", "ValleyAudiodrwav_open_memory");
    build.define("modeldrwav_open_memory", "modelValleyAudiodrwav_open_memory");
    build.define("drwav_open_memoryWidget", "ValleyAudiodrwav_open_memoryWidget");
    build.define("drwav_open_memory_and_read_f32", "ValleyAudiodrwav_open_memory_and_read_f32");
    build.define("modeldrwav_open_memory_and_read_f32", "modelValleyAudiodrwav_open_memory_and_read_f32");
    build.define("drwav_open_memory_and_read_f32Widget", "ValleyAudiodrwav_open_memory_and_read_f32Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_f32", "ValleyAudiodrwav_open_memory_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_f32", "modelValleyAudiodrwav_open_memory_and_read_pcm_frames_f32");
    build.define("drwav_open_memory_and_read_pcm_frames_f32Widget", "ValleyAudiodrwav_open_memory_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_s16", "ValleyAudiodrwav_open_memory_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_s16", "modelValleyAudiodrwav_open_memory_and_read_pcm_frames_s16");
    build.define("drwav_open_memory_and_read_pcm_frames_s16Widget", "ValleyAudiodrwav_open_memory_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_s32", "ValleyAudiodrwav_open_memory_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_s32", "modelValleyAudiodrwav_open_memory_and_read_pcm_frames_s32");
    build.define("drwav_open_memory_and_read_pcm_frames_s32Widget", "ValleyAudiodrwav_open_memory_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_memory_and_read_s16", "ValleyAudiodrwav_open_memory_and_read_s16");
    build.define("modeldrwav_open_memory_and_read_s16", "modelValleyAudiodrwav_open_memory_and_read_s16");
    build.define("drwav_open_memory_and_read_s16Widget", "ValleyAudiodrwav_open_memory_and_read_s16Widget");
    build.define("drwav_open_memory_and_read_s32", "ValleyAudiodrwav_open_memory_and_read_s32");
    build.define("modeldrwav_open_memory_and_read_s32", "modelValleyAudiodrwav_open_memory_and_read_s32");
    build.define("drwav_open_memory_and_read_s32Widget", "ValleyAudiodrwav_open_memory_and_read_s32Widget");
    build.define("drwav_open_memory_ex", "ValleyAudiodrwav_open_memory_ex");
    build.define("modeldrwav_open_memory_ex", "modelValleyAudiodrwav_open_memory_ex");
    build.define("drwav_open_memory_exWidget", "ValleyAudiodrwav_open_memory_exWidget");
    build.define("drwav_open_memory_write", "ValleyAudiodrwav_open_memory_write");
    build.define("modeldrwav_open_memory_write", "modelValleyAudiodrwav_open_memory_write");
    build.define("drwav_open_memory_writeWidget", "ValleyAudiodrwav_open_memory_writeWidget");
    build.define("drwav_open_memory_write", "ValleyAudiodrwav_open_memory_write");
    build.define("modeldrwav_open_memory_write", "modelValleyAudiodrwav_open_memory_write");
    build.define("drwav_open_memory_writeWidget", "ValleyAudiodrwav_open_memory_writeWidget");
    build.define("drwav_open_memory_write__internal", "ValleyAudiodrwav_open_memory_write__internal");
    build.define("modeldrwav_open_memory_write__internal", "modelValleyAudiodrwav_open_memory_write__internal");
    build.define("drwav_open_memory_write__internalWidget", "ValleyAudiodrwav_open_memory_write__internalWidget");
    build.define("drwav_open_memory_write__internal", "ValleyAudiodrwav_open_memory_write__internal");
    build.define("modeldrwav_open_memory_write__internal", "modelValleyAudiodrwav_open_memory_write__internal");
    build.define("drwav_open_memory_write__internalWidget", "ValleyAudiodrwav_open_memory_write__internalWidget");
    build.define("drwav_open_memory_write_sequential", "ValleyAudiodrwav_open_memory_write_sequential");
    build.define("modeldrwav_open_memory_write_sequential", "modelValleyAudiodrwav_open_memory_write_sequential");
    build.define("drwav_open_memory_write_sequentialWidget", "ValleyAudiodrwav_open_memory_write_sequentialWidget");
    build.define("drwav_open_write", "ValleyAudiodrwav_open_write");
    build.define("modeldrwav_open_write", "modelValleyAudiodrwav_open_write");
    build.define("drwav_open_writeWidget", "ValleyAudiodrwav_open_writeWidget");
    build.define("drwav_open_write", "ValleyAudiodrwav_open_write");
    build.define("modeldrwav_open_write", "modelValleyAudiodrwav_open_write");
    build.define("drwav_open_writeWidget", "ValleyAudiodrwav_open_writeWidget");
    build.define("drwav_open_write__internal", "ValleyAudiodrwav_open_write__internal");
    build.define("modeldrwav_open_write__internal", "modelValleyAudiodrwav_open_write__internal");
    build.define("drwav_open_write__internalWidget", "ValleyAudiodrwav_open_write__internalWidget");
    build.define("drwav_open_write_sequential", "ValleyAudiodrwav_open_write_sequential");
    build.define("modeldrwav_open_write_sequential", "modelValleyAudiodrwav_open_write_sequential");
    build.define("drwav_open_write_sequentialWidget", "ValleyAudiodrwav_open_write_sequentialWidget");
    build.define("drwav_read", "ValleyAudiodrwav_read");
    build.define("modeldrwav_read", "modelValleyAudiodrwav_read");
    build.define("drwav_readWidget", "ValleyAudiodrwav_readWidget");
    build.define("drwav_read_f32", "ValleyAudiodrwav_read_f32");
    build.define("modeldrwav_read_f32", "modelValleyAudiodrwav_read_f32");
    build.define("drwav_read_f32Widget", "ValleyAudiodrwav_read_f32Widget");
    build.define("drwav_read_f32__alaw", "ValleyAudiodrwav_read_f32__alaw");
    build.define("modeldrwav_read_f32__alaw", "modelValleyAudiodrwav_read_f32__alaw");
    build.define("drwav_read_f32__alawWidget", "ValleyAudiodrwav_read_f32__alawWidget");
    build.define("drwav_read_f32__alaw", "ValleyAudiodrwav_read_f32__alaw");
    build.define("modeldrwav_read_f32__alaw", "modelValleyAudiodrwav_read_f32__alaw");
    build.define("drwav_read_f32__alawWidget", "ValleyAudiodrwav_read_f32__alawWidget");
    build.define("drwav_read_f32__ieee", "ValleyAudiodrwav_read_f32__ieee");
    build.define("modeldrwav_read_f32__ieee", "modelValleyAudiodrwav_read_f32__ieee");
    build.define("drwav_read_f32__ieeeWidget", "ValleyAudiodrwav_read_f32__ieeeWidget");
    build.define("drwav_read_f32__ieee", "ValleyAudiodrwav_read_f32__ieee");
    build.define("modeldrwav_read_f32__ieee", "modelValleyAudiodrwav_read_f32__ieee");
    build.define("drwav_read_f32__ieeeWidget", "ValleyAudiodrwav_read_f32__ieeeWidget");
    build.define("drwav_read_f32__ima", "ValleyAudiodrwav_read_f32__ima");
    build.define("modeldrwav_read_f32__ima", "modelValleyAudiodrwav_read_f32__ima");
    build.define("drwav_read_f32__imaWidget", "ValleyAudiodrwav_read_f32__imaWidget");
    build.define("drwav_read_f32__ima", "ValleyAudiodrwav_read_f32__ima");
    build.define("modeldrwav_read_f32__ima", "modelValleyAudiodrwav_read_f32__ima");
    build.define("drwav_read_f32__imaWidget", "ValleyAudiodrwav_read_f32__imaWidget");
    build.define("drwav_read_f32__msadpcm", "ValleyAudiodrwav_read_f32__msadpcm");
    build.define("modeldrwav_read_f32__msadpcm", "modelValleyAudiodrwav_read_f32__msadpcm");
    build.define("drwav_read_f32__msadpcmWidget", "ValleyAudiodrwav_read_f32__msadpcmWidget");
    build.define("drwav_read_f32__msadpcm", "ValleyAudiodrwav_read_f32__msadpcm");
    build.define("modeldrwav_read_f32__msadpcm", "modelValleyAudiodrwav_read_f32__msadpcm");
    build.define("drwav_read_f32__msadpcmWidget", "ValleyAudiodrwav_read_f32__msadpcmWidget");
    build.define("drwav_read_f32__mulaw", "ValleyAudiodrwav_read_f32__mulaw");
    build.define("modeldrwav_read_f32__mulaw", "modelValleyAudiodrwav_read_f32__mulaw");
    build.define("drwav_read_f32__mulawWidget", "ValleyAudiodrwav_read_f32__mulawWidget");
    build.define("drwav_read_f32__mulaw", "ValleyAudiodrwav_read_f32__mulaw");
    build.define("modeldrwav_read_f32__mulaw", "modelValleyAudiodrwav_read_f32__mulaw");
    build.define("drwav_read_f32__mulawWidget", "ValleyAudiodrwav_read_f32__mulawWidget");
    build.define("drwav_read_f32__pcm", "ValleyAudiodrwav_read_f32__pcm");
    build.define("modeldrwav_read_f32__pcm", "modelValleyAudiodrwav_read_f32__pcm");
    build.define("drwav_read_f32__pcmWidget", "ValleyAudiodrwav_read_f32__pcmWidget");
    build.define("drwav_read_f32__pcm", "ValleyAudiodrwav_read_f32__pcm");
    build.define("modeldrwav_read_f32__pcm", "modelValleyAudiodrwav_read_f32__pcm");
    build.define("drwav_read_f32__pcmWidget", "ValleyAudiodrwav_read_f32__pcmWidget");
    build.define("drwav_read_pcm_frames", "ValleyAudiodrwav_read_pcm_frames");
    build.define("modeldrwav_read_pcm_frames", "modelValleyAudiodrwav_read_pcm_frames");
    build.define("drwav_read_pcm_framesWidget", "ValleyAudiodrwav_read_pcm_framesWidget");
    build.define("drwav_read_pcm_frames_be", "ValleyAudiodrwav_read_pcm_frames_be");
    build.define("modeldrwav_read_pcm_frames_be", "modelValleyAudiodrwav_read_pcm_frames_be");
    build.define("drwav_read_pcm_frames_beWidget", "ValleyAudiodrwav_read_pcm_frames_beWidget");
    build.define("drwav_read_pcm_frames_f32", "ValleyAudiodrwav_read_pcm_frames_f32");
    build.define("modeldrwav_read_pcm_frames_f32", "modelValleyAudiodrwav_read_pcm_frames_f32");
    build.define("drwav_read_pcm_frames_f32Widget", "ValleyAudiodrwav_read_pcm_frames_f32Widget");
    build.define("drwav_read_pcm_frames_f32be", "ValleyAudiodrwav_read_pcm_frames_f32be");
    build.define("modeldrwav_read_pcm_frames_f32be", "modelValleyAudiodrwav_read_pcm_frames_f32be");
    build.define("drwav_read_pcm_frames_f32beWidget", "ValleyAudiodrwav_read_pcm_frames_f32beWidget");
    build.define("drwav_read_pcm_frames_f32le", "ValleyAudiodrwav_read_pcm_frames_f32le");
    build.define("modeldrwav_read_pcm_frames_f32le", "modelValleyAudiodrwav_read_pcm_frames_f32le");
    build.define("drwav_read_pcm_frames_f32leWidget", "ValleyAudiodrwav_read_pcm_frames_f32leWidget");
    build.define("drwav_read_pcm_frames_le", "ValleyAudiodrwav_read_pcm_frames_le");
    build.define("modeldrwav_read_pcm_frames_le", "modelValleyAudiodrwav_read_pcm_frames_le");
    build.define("drwav_read_pcm_frames_leWidget", "ValleyAudiodrwav_read_pcm_frames_leWidget");
    build.define("drwav_read_pcm_frames_s16", "ValleyAudiodrwav_read_pcm_frames_s16");
    build.define("modeldrwav_read_pcm_frames_s16", "modelValleyAudiodrwav_read_pcm_frames_s16");
    build.define("drwav_read_pcm_frames_s16Widget", "ValleyAudiodrwav_read_pcm_frames_s16Widget");
    build.define("drwav_read_pcm_frames_s16be", "ValleyAudiodrwav_read_pcm_frames_s16be");
    build.define("modeldrwav_read_pcm_frames_s16be", "modelValleyAudiodrwav_read_pcm_frames_s16be");
    build.define("drwav_read_pcm_frames_s16beWidget", "ValleyAudiodrwav_read_pcm_frames_s16beWidget");
    build.define("drwav_read_pcm_frames_s16le", "ValleyAudiodrwav_read_pcm_frames_s16le");
    build.define("modeldrwav_read_pcm_frames_s16le", "modelValleyAudiodrwav_read_pcm_frames_s16le");
    build.define("drwav_read_pcm_frames_s16leWidget", "ValleyAudiodrwav_read_pcm_frames_s16leWidget");
    build.define("drwav_read_pcm_frames_s32", "ValleyAudiodrwav_read_pcm_frames_s32");
    build.define("modeldrwav_read_pcm_frames_s32", "modelValleyAudiodrwav_read_pcm_frames_s32");
    build.define("drwav_read_pcm_frames_s32Widget", "ValleyAudiodrwav_read_pcm_frames_s32Widget");
    build.define("drwav_read_pcm_frames_s32be", "ValleyAudiodrwav_read_pcm_frames_s32be");
    build.define("modeldrwav_read_pcm_frames_s32be", "modelValleyAudiodrwav_read_pcm_frames_s32be");
    build.define("drwav_read_pcm_frames_s32beWidget", "ValleyAudiodrwav_read_pcm_frames_s32beWidget");
    build.define("drwav_read_pcm_frames_s32le", "ValleyAudiodrwav_read_pcm_frames_s32le");
    build.define("modeldrwav_read_pcm_frames_s32le", "modelValleyAudiodrwav_read_pcm_frames_s32le");
    build.define("drwav_read_pcm_frames_s32leWidget", "ValleyAudiodrwav_read_pcm_frames_s32leWidget");
    build.define("drwav_read_raw", "ValleyAudiodrwav_read_raw");
    build.define("modeldrwav_read_raw", "modelValleyAudiodrwav_read_raw");
    build.define("drwav_read_rawWidget", "ValleyAudiodrwav_read_rawWidget");
    build.define("drwav_read_s16", "ValleyAudiodrwav_read_s16");
    build.define("modeldrwav_read_s16", "modelValleyAudiodrwav_read_s16");
    build.define("drwav_read_s16Widget", "ValleyAudiodrwav_read_s16Widget");
    build.define("drwav_read_s16__alaw", "ValleyAudiodrwav_read_s16__alaw");
    build.define("modeldrwav_read_s16__alaw", "modelValleyAudiodrwav_read_s16__alaw");
    build.define("drwav_read_s16__alawWidget", "ValleyAudiodrwav_read_s16__alawWidget");
    build.define("drwav_read_s16__ieee", "ValleyAudiodrwav_read_s16__ieee");
    build.define("modeldrwav_read_s16__ieee", "modelValleyAudiodrwav_read_s16__ieee");
    build.define("drwav_read_s16__ieeeWidget", "ValleyAudiodrwav_read_s16__ieeeWidget");
    build.define("drwav_read_s16__ima", "ValleyAudiodrwav_read_s16__ima");
    build.define("modeldrwav_read_s16__ima", "modelValleyAudiodrwav_read_s16__ima");
    build.define("drwav_read_s16__imaWidget", "ValleyAudiodrwav_read_s16__imaWidget");
    build.define("drwav_read_s16__msadpcm", "ValleyAudiodrwav_read_s16__msadpcm");
    build.define("modeldrwav_read_s16__msadpcm", "modelValleyAudiodrwav_read_s16__msadpcm");
    build.define("drwav_read_s16__msadpcmWidget", "ValleyAudiodrwav_read_s16__msadpcmWidget");
    build.define("drwav_read_s16__mulaw", "ValleyAudiodrwav_read_s16__mulaw");
    build.define("modeldrwav_read_s16__mulaw", "modelValleyAudiodrwav_read_s16__mulaw");
    build.define("drwav_read_s16__mulawWidget", "ValleyAudiodrwav_read_s16__mulawWidget");
    build.define("drwav_read_s16__pcm", "ValleyAudiodrwav_read_s16__pcm");
    build.define("modeldrwav_read_s16__pcm", "modelValleyAudiodrwav_read_s16__pcm");
    build.define("drwav_read_s16__pcmWidget", "ValleyAudiodrwav_read_s16__pcmWidget");
    build.define("drwav_read_s32", "ValleyAudiodrwav_read_s32");
    build.define("modeldrwav_read_s32", "modelValleyAudiodrwav_read_s32");
    build.define("drwav_read_s32Widget", "ValleyAudiodrwav_read_s32Widget");
    build.define("drwav_read_s32__alaw", "ValleyAudiodrwav_read_s32__alaw");
    build.define("modeldrwav_read_s32__alaw", "modelValleyAudiodrwav_read_s32__alaw");
    build.define("drwav_read_s32__alawWidget", "ValleyAudiodrwav_read_s32__alawWidget");
    build.define("drwav_read_s32__alaw", "ValleyAudiodrwav_read_s32__alaw");
    build.define("modeldrwav_read_s32__alaw", "modelValleyAudiodrwav_read_s32__alaw");
    build.define("drwav_read_s32__alawWidget", "ValleyAudiodrwav_read_s32__alawWidget");
    build.define("drwav_read_s32__ieee", "ValleyAudiodrwav_read_s32__ieee");
    build.define("modeldrwav_read_s32__ieee", "modelValleyAudiodrwav_read_s32__ieee");
    build.define("drwav_read_s32__ieeeWidget", "ValleyAudiodrwav_read_s32__ieeeWidget");
    build.define("drwav_read_s32__ieee", "ValleyAudiodrwav_read_s32__ieee");
    build.define("modeldrwav_read_s32__ieee", "modelValleyAudiodrwav_read_s32__ieee");
    build.define("drwav_read_s32__ieeeWidget", "ValleyAudiodrwav_read_s32__ieeeWidget");
    build.define("drwav_read_s32__ima", "ValleyAudiodrwav_read_s32__ima");
    build.define("modeldrwav_read_s32__ima", "modelValleyAudiodrwav_read_s32__ima");
    build.define("drwav_read_s32__imaWidget", "ValleyAudiodrwav_read_s32__imaWidget");
    build.define("drwav_read_s32__ima", "ValleyAudiodrwav_read_s32__ima");
    build.define("modeldrwav_read_s32__ima", "modelValleyAudiodrwav_read_s32__ima");
    build.define("drwav_read_s32__imaWidget", "ValleyAudiodrwav_read_s32__imaWidget");
    build.define("drwav_read_s32__msadpcm", "ValleyAudiodrwav_read_s32__msadpcm");
    build.define("modeldrwav_read_s32__msadpcm", "modelValleyAudiodrwav_read_s32__msadpcm");
    build.define("drwav_read_s32__msadpcmWidget", "ValleyAudiodrwav_read_s32__msadpcmWidget");
    build.define("drwav_read_s32__msadpcm", "ValleyAudiodrwav_read_s32__msadpcm");
    build.define("modeldrwav_read_s32__msadpcm", "modelValleyAudiodrwav_read_s32__msadpcm");
    build.define("drwav_read_s32__msadpcmWidget", "ValleyAudiodrwav_read_s32__msadpcmWidget");
    build.define("drwav_read_s32__mulaw", "ValleyAudiodrwav_read_s32__mulaw");
    build.define("modeldrwav_read_s32__mulaw", "modelValleyAudiodrwav_read_s32__mulaw");
    build.define("drwav_read_s32__mulawWidget", "ValleyAudiodrwav_read_s32__mulawWidget");
    build.define("drwav_read_s32__mulaw", "ValleyAudiodrwav_read_s32__mulaw");
    build.define("modeldrwav_read_s32__mulaw", "modelValleyAudiodrwav_read_s32__mulaw");
    build.define("drwav_read_s32__mulawWidget", "ValleyAudiodrwav_read_s32__mulawWidget");
    build.define("drwav_read_s32__pcm", "ValleyAudiodrwav_read_s32__pcm");
    build.define("modeldrwav_read_s32__pcm", "modelValleyAudiodrwav_read_s32__pcm");
    build.define("drwav_read_s32__pcmWidget", "ValleyAudiodrwav_read_s32__pcmWidget");
    build.define("drwav_read_s32__pcm", "ValleyAudiodrwav_read_s32__pcm");
    build.define("modeldrwav_read_s32__pcm", "modelValleyAudiodrwav_read_s32__pcm");
    build.define("drwav_read_s32__pcmWidget", "ValleyAudiodrwav_read_s32__pcmWidget");
    build.define("drwav_riff_chunk_size_riff", "ValleyAudiodrwav_riff_chunk_size_riff");
    build.define("modeldrwav_riff_chunk_size_riff", "modelValleyAudiodrwav_riff_chunk_size_riff");
    build.define("drwav_riff_chunk_size_riffWidget", "ValleyAudiodrwav_riff_chunk_size_riffWidget");
    build.define("drwav_riff_chunk_size_w64", "ValleyAudiodrwav_riff_chunk_size_w64");
    build.define("modeldrwav_riff_chunk_size_w64", "modelValleyAudiodrwav_riff_chunk_size_w64");
    build.define("drwav_riff_chunk_size_w64Widget", "ValleyAudiodrwav_riff_chunk_size_w64Widget");
    build.define("drwav_s16_to_f32", "ValleyAudiodrwav_s16_to_f32");
    build.define("modeldrwav_s16_to_f32", "modelValleyAudiodrwav_s16_to_f32");
    build.define("drwav_s16_to_f32Widget", "ValleyAudiodrwav_s16_to_f32Widget");
    build.define("drwav_s16_to_s32", "ValleyAudiodrwav_s16_to_s32");
    build.define("modeldrwav_s16_to_s32", "modelValleyAudiodrwav_s16_to_s32");
    build.define("drwav_s16_to_s32Widget", "ValleyAudiodrwav_s16_to_s32Widget");
    build.define("drwav_s24_to_f32", "ValleyAudiodrwav_s24_to_f32");
    build.define("modeldrwav_s24_to_f32", "modelValleyAudiodrwav_s24_to_f32");
    build.define("drwav_s24_to_f32Widget", "ValleyAudiodrwav_s24_to_f32Widget");
    build.define("drwav_s24_to_s16", "ValleyAudiodrwav_s24_to_s16");
    build.define("modeldrwav_s24_to_s16", "modelValleyAudiodrwav_s24_to_s16");
    build.define("drwav_s24_to_s16Widget", "ValleyAudiodrwav_s24_to_s16Widget");
    build.define("drwav_s24_to_s16", "ValleyAudiodrwav_s24_to_s16");
    build.define("modeldrwav_s24_to_s16", "modelValleyAudiodrwav_s24_to_s16");
    build.define("drwav_s24_to_s16Widget", "ValleyAudiodrwav_s24_to_s16Widget");
    build.define("drwav_s24_to_s32", "ValleyAudiodrwav_s24_to_s32");
    build.define("modeldrwav_s24_to_s32", "modelValleyAudiodrwav_s24_to_s32");
    build.define("drwav_s24_to_s32Widget", "ValleyAudiodrwav_s24_to_s32Widget");
    build.define("drwav_s32_to_f32", "ValleyAudiodrwav_s32_to_f32");
    build.define("modeldrwav_s32_to_f32", "modelValleyAudiodrwav_s32_to_f32");
    build.define("drwav_s32_to_f32Widget", "ValleyAudiodrwav_s32_to_f32Widget");
    build.define("drwav_s32_to_s16", "ValleyAudiodrwav_s32_to_s16");
    build.define("modeldrwav_s32_to_s16", "modelValleyAudiodrwav_s32_to_s16");
    build.define("drwav_s32_to_s16Widget", "ValleyAudiodrwav_s32_to_s16Widget");
    build.define("drwav_s32_to_s16", "ValleyAudiodrwav_s32_to_s16");
    build.define("modeldrwav_s32_to_s16", "modelValleyAudiodrwav_s32_to_s16");
    build.define("drwav_s32_to_s16Widget", "ValleyAudiodrwav_s32_to_s16Widget");
    build.define("drwav_seek_to_pcm_frame", "ValleyAudiodrwav_seek_to_pcm_frame");
    build.define("modeldrwav_seek_to_pcm_frame", "modelValleyAudiodrwav_seek_to_pcm_frame");
    build.define("drwav_seek_to_pcm_frameWidget", "ValleyAudiodrwav_seek_to_pcm_frameWidget");
    build.define("drwav_seek_to_sample", "ValleyAudiodrwav_seek_to_sample");
    build.define("modeldrwav_seek_to_sample", "modelValleyAudiodrwav_seek_to_sample");
    build.define("drwav_seek_to_sampleWidget", "ValleyAudiodrwav_seek_to_sampleWidget");
    build.define("drwav_seek_to_sample", "ValleyAudiodrwav_seek_to_sample");
    build.define("modeldrwav_seek_to_sample", "modelValleyAudiodrwav_seek_to_sample");
    build.define("drwav_seek_to_sampleWidget", "ValleyAudiodrwav_seek_to_sampleWidget");
    build.define("drwav_smpl", "ValleyAudiodrwav_smpl");
    build.define("modeldrwav_smpl", "modelValleyAudiodrwav_smpl");
    build.define("drwav_smplWidget", "ValleyAudiodrwav_smplWidget");
    build.define("drwav_smpl_loop", "ValleyAudiodrwav_smpl_loop");
    build.define("modeldrwav_smpl_loop", "modelValleyAudiodrwav_smpl_loop");
    build.define("drwav_smpl_loopWidget", "ValleyAudiodrwav_smpl_loopWidget");
    build.define("drwav_take_ownership_of_metadata", "ValleyAudiodrwav_take_ownership_of_metadata");
    build.define("modeldrwav_take_ownership_of_metadata", "modelValleyAudiodrwav_take_ownership_of_metadata");
    build.define("drwav_take_ownership_of_metadataWidget", "ValleyAudiodrwav_take_ownership_of_metadataWidget");
    build.define("drwav_target_write_size_bytes", "ValleyAudiodrwav_target_write_size_bytes");
    build.define("modeldrwav_target_write_size_bytes", "modelValleyAudiodrwav_target_write_size_bytes");
    build.define("drwav_target_write_size_bytesWidget", "ValleyAudiodrwav_target_write_size_bytesWidget");
    build.define("drwav_u8_to_f32", "ValleyAudiodrwav_u8_to_f32");
    build.define("modeldrwav_u8_to_f32", "modelValleyAudiodrwav_u8_to_f32");
    build.define("drwav_u8_to_f32Widget", "ValleyAudiodrwav_u8_to_f32Widget");
    build.define("drwav_u8_to_s16", "ValleyAudiodrwav_u8_to_s16");
    build.define("modeldrwav_u8_to_s16", "modelValleyAudiodrwav_u8_to_s16");
    build.define("drwav_u8_to_s16Widget", "ValleyAudiodrwav_u8_to_s16Widget");
    build.define("drwav_u8_to_s16", "ValleyAudiodrwav_u8_to_s16");
    build.define("modeldrwav_u8_to_s16", "modelValleyAudiodrwav_u8_to_s16");
    build.define("drwav_u8_to_s16Widget", "ValleyAudiodrwav_u8_to_s16Widget");
    build.define("drwav_u8_to_s32", "ValleyAudiodrwav_u8_to_s32");
    build.define("modeldrwav_u8_to_s32", "modelValleyAudiodrwav_u8_to_s32");
    build.define("drwav_u8_to_s32Widget", "ValleyAudiodrwav_u8_to_s32Widget");
    build.define("drwav_uninit", "ValleyAudiodrwav_uninit");
    build.define("modeldrwav_uninit", "modelValleyAudiodrwav_uninit");
    build.define("drwav_uninitWidget", "ValleyAudiodrwav_uninitWidget");
    build.define("drwav_version", "ValleyAudiodrwav_version");
    build.define("modeldrwav_version", "modelValleyAudiodrwav_version");
    build.define("drwav_versionWidget", "ValleyAudiodrwav_versionWidget");
    build.define("drwav_version_string", "ValleyAudiodrwav_version_string");
    build.define("modeldrwav_version_string", "modelValleyAudiodrwav_version_string");
    build.define("drwav_version_stringWidget", "ValleyAudiodrwav_version_stringWidget");
    build.define("drwav_write", "ValleyAudiodrwav_write");
    build.define("modeldrwav_write", "modelValleyAudiodrwav_write");
    build.define("drwav_writeWidget", "ValleyAudiodrwav_writeWidget");
    build.define("drwav_write", "ValleyAudiodrwav_write");
    build.define("modeldrwav_write", "modelValleyAudiodrwav_write");
    build.define("drwav_writeWidget", "ValleyAudiodrwav_writeWidget");
    build.define("drwav_write_pcm_frames", "ValleyAudiodrwav_write_pcm_frames");
    build.define("modeldrwav_write_pcm_frames", "modelValleyAudiodrwav_write_pcm_frames");
    build.define("drwav_write_pcm_framesWidget", "ValleyAudiodrwav_write_pcm_framesWidget");
    build.define("drwav_write_pcm_frames_be", "ValleyAudiodrwav_write_pcm_frames_be");
    build.define("modeldrwav_write_pcm_frames_be", "modelValleyAudiodrwav_write_pcm_frames_be");
    build.define("drwav_write_pcm_frames_beWidget", "ValleyAudiodrwav_write_pcm_frames_beWidget");
    build.define("drwav_write_pcm_frames_le", "ValleyAudiodrwav_write_pcm_frames_le");
    build.define("modeldrwav_write_pcm_frames_le", "modelValleyAudiodrwav_write_pcm_frames_le");
    build.define("drwav_write_pcm_frames_leWidget", "ValleyAudiodrwav_write_pcm_frames_leWidget");
    build.define("drwav_write_raw", "ValleyAudiodrwav_write_raw");
    build.define("modeldrwav_write_raw", "modelValleyAudiodrwav_write_raw");
    build.define("drwav_write_rawWidget", "ValleyAudiodrwav_write_rawWidget");
    build.define("Chord", "ValleyAudioChord");
    build.define("modelChord", "modelValleyAudioChord");
    build.define("ChordWidget", "ValleyAudioChordWidget");
    build.define("DigitalDisplay", "ValleyAudioDigitalDisplay");
    build.define("modelDigitalDisplay", "modelValleyAudioDigitalDisplay");
    build.define("DigitalDisplayWidget", "ValleyAudioDigitalDisplayWidget");

    // Filter-out list
    let filter_out: Vec<String> = vec![
        "ValleyAudio/src/Valley.cpp".to_string(),
    ];

    // Source files

    // Glob ValleyAudio/src/**/*.cpp|cc|c (recursive)
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
    collect_sources(&plugins_dir.join("ValleyAudio/src"), &filter_out, &plugins_dir, &mut build, 0);

    // Glob ValleyAudio/src/*/**/*.cpp|cc|c (recursive)
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
    collect_sources(&plugins_dir.join("ValleyAudio/src/*"), &filter_out, &plugins_dir, &mut build, 0);

    // Glob ValleyAudio/src/*/*/**/*.cpp|cc|c (recursive)
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
    collect_sources(&plugins_dir.join("ValleyAudio/src/*/*"), &filter_out, &plugins_dir, &mut build, 0);

    build.compile("cardinal_plugin_valleyaudio");
}
