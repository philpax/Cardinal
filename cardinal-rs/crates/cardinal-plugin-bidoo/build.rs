use std::path::PathBuf;

fn main() {
    let cardinal_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // cardinal-rs/
        .parent().unwrap()  // Cardinal/
        .to_path_buf();

    let plugins_dir = cardinal_root.join("plugins");
    let plugin_dir = plugins_dir.join("Bidoo");

    if !plugin_dir.exists() {
        eprintln!("Plugin Bidoo not found (submodule not initialized?), skipping");
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
    build.define("pluginInstance", "pluginInstance__Bidoo");
    build.define("init", "init__Bidoo");
    build.define("ChannelDisplay", "BidooChannelDisplay");
    build.define("modelChannelDisplay", "modelBidooChannelDisplay");
    build.define("ChannelDisplayWidget", "BidooChannelDisplayWidget");
    build.define("InstantiateExpanderItem", "BidooInstantiateExpanderItem");
    build.define("modelInstantiateExpanderItem", "modelBidooInstantiateExpanderItem");
    build.define("InstantiateExpanderItemWidget", "BidooInstantiateExpanderItemWidget");
    build.define("LadderFilter", "BidooLadderFilter");
    build.define("modelLadderFilter", "modelBidooLadderFilter");
    build.define("LadderFilterWidget", "BidooLadderFilterWidget");
    build.define("PitchShifter", "BidooPitchShifter");
    build.define("modelPitchShifter", "modelBidooPitchShifter");
    build.define("PitchShifterWidget", "BidooPitchShifterWidget");
    build.define("drwav", "Bidoodrwav");
    build.define("modeldrwav", "modelBidoodrwav");
    build.define("drwavWidget", "BidoodrwavWidget");
    build.define("drwav__on_read", "Bidoodrwav__on_read");
    build.define("modeldrwav__on_read", "modelBidoodrwav__on_read");
    build.define("drwav__on_readWidget", "Bidoodrwav__on_readWidget");
    build.define("drwav__on_seek", "Bidoodrwav__on_seek");
    build.define("modeldrwav__on_seek", "modelBidoodrwav__on_seek");
    build.define("drwav__on_seekWidget", "Bidoodrwav__on_seekWidget");
    build.define("drwav__read_and_close_f32", "Bidoodrwav__read_and_close_f32");
    build.define("modeldrwav__read_and_close_f32", "modelBidoodrwav__read_and_close_f32");
    build.define("drwav__read_and_close_f32Widget", "Bidoodrwav__read_and_close_f32Widget");
    build.define("drwav__read_and_close_s16", "Bidoodrwav__read_and_close_s16");
    build.define("modeldrwav__read_and_close_s16", "modelBidoodrwav__read_and_close_s16");
    build.define("drwav__read_and_close_s16Widget", "Bidoodrwav__read_and_close_s16Widget");
    build.define("drwav__read_and_close_s32", "Bidoodrwav__read_and_close_s32");
    build.define("modeldrwav__read_and_close_s32", "modelBidoodrwav__read_and_close_s32");
    build.define("drwav__read_and_close_s32Widget", "Bidoodrwav__read_and_close_s32Widget");
    build.define("drwav_alaw_to_f32", "Bidoodrwav_alaw_to_f32");
    build.define("modeldrwav_alaw_to_f32", "modelBidoodrwav_alaw_to_f32");
    build.define("drwav_alaw_to_f32Widget", "Bidoodrwav_alaw_to_f32Widget");
    build.define("drwav_alaw_to_s16", "Bidoodrwav_alaw_to_s16");
    build.define("modeldrwav_alaw_to_s16", "modelBidoodrwav_alaw_to_s16");
    build.define("drwav_alaw_to_s16Widget", "Bidoodrwav_alaw_to_s16Widget");
    build.define("drwav_alaw_to_s16", "Bidoodrwav_alaw_to_s16");
    build.define("modeldrwav_alaw_to_s16", "modelBidoodrwav_alaw_to_s16");
    build.define("drwav_alaw_to_s16Widget", "Bidoodrwav_alaw_to_s16Widget");
    build.define("drwav_alaw_to_s32", "Bidoodrwav_alaw_to_s32");
    build.define("modeldrwav_alaw_to_s32", "modelBidoodrwav_alaw_to_s32");
    build.define("drwav_alaw_to_s32Widget", "Bidoodrwav_alaw_to_s32Widget");
    build.define("drwav_bytes_to_f32", "Bidoodrwav_bytes_to_f32");
    build.define("modeldrwav_bytes_to_f32", "modelBidoodrwav_bytes_to_f32");
    build.define("drwav_bytes_to_f32Widget", "Bidoodrwav_bytes_to_f32Widget");
    build.define("drwav_bytes_to_s16", "Bidoodrwav_bytes_to_s16");
    build.define("modeldrwav_bytes_to_s16", "modelBidoodrwav_bytes_to_s16");
    build.define("drwav_bytes_to_s16Widget", "Bidoodrwav_bytes_to_s16Widget");
    build.define("drwav_bytes_to_s32", "Bidoodrwav_bytes_to_s32");
    build.define("modeldrwav_bytes_to_s32", "modelBidoodrwav_bytes_to_s32");
    build.define("drwav_bytes_to_s32Widget", "Bidoodrwav_bytes_to_s32Widget");
    build.define("drwav_bytes_to_s64", "Bidoodrwav_bytes_to_s64");
    build.define("modeldrwav_bytes_to_s64", "modelBidoodrwav_bytes_to_s64");
    build.define("drwav_bytes_to_s64Widget", "Bidoodrwav_bytes_to_s64Widget");
    build.define("drwav_bytes_to_u16", "Bidoodrwav_bytes_to_u16");
    build.define("modeldrwav_bytes_to_u16", "modelBidoodrwav_bytes_to_u16");
    build.define("drwav_bytes_to_u16Widget", "Bidoodrwav_bytes_to_u16Widget");
    build.define("drwav_bytes_to_u32", "Bidoodrwav_bytes_to_u32");
    build.define("modeldrwav_bytes_to_u32", "modelBidoodrwav_bytes_to_u32");
    build.define("drwav_bytes_to_u32Widget", "Bidoodrwav_bytes_to_u32Widget");
    build.define("drwav_bytes_to_u64", "Bidoodrwav_bytes_to_u64");
    build.define("modeldrwav_bytes_to_u64", "modelBidoodrwav_bytes_to_u64");
    build.define("drwav_bytes_to_u64Widget", "Bidoodrwav_bytes_to_u64Widget");
    build.define("drwav_close", "Bidoodrwav_close");
    build.define("modeldrwav_close", "modelBidoodrwav_close");
    build.define("drwav_closeWidget", "Bidoodrwav_closeWidget");
    build.define("drwav_close", "Bidoodrwav_close");
    build.define("modeldrwav_close", "modelBidoodrwav_close");
    build.define("drwav_closeWidget", "Bidoodrwav_closeWidget");
    build.define("drwav_container", "Bidoodrwav_container");
    build.define("modeldrwav_container", "modelBidoodrwav_container");
    build.define("drwav_containerWidget", "Bidoodrwav_containerWidget");
    build.define("drwav_data_chunk_size_riff", "Bidoodrwav_data_chunk_size_riff");
    build.define("modeldrwav_data_chunk_size_riff", "modelBidoodrwav_data_chunk_size_riff");
    build.define("drwav_data_chunk_size_riffWidget", "Bidoodrwav_data_chunk_size_riffWidget");
    build.define("drwav_data_chunk_size_w64", "Bidoodrwav_data_chunk_size_w64");
    build.define("modeldrwav_data_chunk_size_w64", "modelBidoodrwav_data_chunk_size_w64");
    build.define("drwav_data_chunk_size_w64Widget", "Bidoodrwav_data_chunk_size_w64Widget");
    build.define("drwav_data_format", "Bidoodrwav_data_format");
    build.define("modeldrwav_data_format", "modelBidoodrwav_data_format");
    build.define("drwav_data_formatWidget", "Bidoodrwav_data_formatWidget");
    build.define("drwav_f32_to_s16", "Bidoodrwav_f32_to_s16");
    build.define("modeldrwav_f32_to_s16", "modelBidoodrwav_f32_to_s16");
    build.define("drwav_f32_to_s16Widget", "Bidoodrwav_f32_to_s16Widget");
    build.define("drwav_f32_to_s32", "Bidoodrwav_f32_to_s32");
    build.define("modeldrwav_f32_to_s32", "modelBidoodrwav_f32_to_s32");
    build.define("drwav_f32_to_s32Widget", "Bidoodrwav_f32_to_s32Widget");
    build.define("drwav_f64_to_f32", "Bidoodrwav_f64_to_f32");
    build.define("modeldrwav_f64_to_f32", "modelBidoodrwav_f64_to_f32");
    build.define("drwav_f64_to_f32Widget", "Bidoodrwav_f64_to_f32Widget");
    build.define("drwav_f64_to_s16", "Bidoodrwav_f64_to_s16");
    build.define("modeldrwav_f64_to_s16", "modelBidoodrwav_f64_to_s16");
    build.define("drwav_f64_to_s16Widget", "Bidoodrwav_f64_to_s16Widget");
    build.define("drwav_f64_to_s16", "Bidoodrwav_f64_to_s16");
    build.define("modeldrwav_f64_to_s16", "modelBidoodrwav_f64_to_s16");
    build.define("drwav_f64_to_s16Widget", "Bidoodrwav_f64_to_s16Widget");
    build.define("drwav_f64_to_s32", "Bidoodrwav_f64_to_s32");
    build.define("modeldrwav_f64_to_s32", "modelBidoodrwav_f64_to_s32");
    build.define("drwav_f64_to_s32Widget", "Bidoodrwav_f64_to_s32Widget");
    build.define("drwav_fmt_get_format", "Bidoodrwav_fmt_get_format");
    build.define("modeldrwav_fmt_get_format", "modelBidoodrwav_fmt_get_format");
    build.define("drwav_fmt_get_formatWidget", "Bidoodrwav_fmt_get_formatWidget");
    build.define("drwav_fopen", "Bidoodrwav_fopen");
    build.define("modeldrwav_fopen", "modelBidoodrwav_fopen");
    build.define("drwav_fopenWidget", "Bidoodrwav_fopenWidget");
    build.define("drwav_fourcc_equal", "Bidoodrwav_fourcc_equal");
    build.define("modeldrwav_fourcc_equal", "modelBidoodrwav_fourcc_equal");
    build.define("drwav_fourcc_equalWidget", "Bidoodrwav_fourcc_equalWidget");
    build.define("drwav_free", "Bidoodrwav_free");
    build.define("modeldrwav_free", "modelBidoodrwav_free");
    build.define("drwav_freeWidget", "Bidoodrwav_freeWidget");
    build.define("drwav_get_cursor_in_pcm_frames", "Bidoodrwav_get_cursor_in_pcm_frames");
    build.define("modeldrwav_get_cursor_in_pcm_frames", "modelBidoodrwav_get_cursor_in_pcm_frames");
    build.define("drwav_get_cursor_in_pcm_framesWidget", "Bidoodrwav_get_cursor_in_pcm_framesWidget");
    build.define("drwav_get_length_in_pcm_frames", "Bidoodrwav_get_length_in_pcm_frames");
    build.define("modeldrwav_get_length_in_pcm_frames", "modelBidoodrwav_get_length_in_pcm_frames");
    build.define("drwav_get_length_in_pcm_framesWidget", "Bidoodrwav_get_length_in_pcm_framesWidget");
    build.define("drwav_guid_equal", "Bidoodrwav_guid_equal");
    build.define("modeldrwav_guid_equal", "modelBidoodrwav_guid_equal");
    build.define("drwav_guid_equalWidget", "Bidoodrwav_guid_equalWidget");
    build.define("drwav_init", "Bidoodrwav_init");
    build.define("modeldrwav_init", "modelBidoodrwav_init");
    build.define("drwav_initWidget", "Bidoodrwav_initWidget");
    build.define("drwav_init_ex", "Bidoodrwav_init_ex");
    build.define("modeldrwav_init_ex", "modelBidoodrwav_init_ex");
    build.define("drwav_init_exWidget", "Bidoodrwav_init_exWidget");
    build.define("drwav_init_file", "Bidoodrwav_init_file");
    build.define("modeldrwav_init_file", "modelBidoodrwav_init_file");
    build.define("drwav_init_fileWidget", "Bidoodrwav_init_fileWidget");
    build.define("drwav_init_file_ex", "Bidoodrwav_init_file_ex");
    build.define("modeldrwav_init_file_ex", "modelBidoodrwav_init_file_ex");
    build.define("drwav_init_file_exWidget", "Bidoodrwav_init_file_exWidget");
    build.define("drwav_init_file_ex_w", "Bidoodrwav_init_file_ex_w");
    build.define("modeldrwav_init_file_ex_w", "modelBidoodrwav_init_file_ex_w");
    build.define("drwav_init_file_ex_wWidget", "Bidoodrwav_init_file_ex_wWidget");
    build.define("drwav_init_file_w", "Bidoodrwav_init_file_w");
    build.define("modeldrwav_init_file_w", "modelBidoodrwav_init_file_w");
    build.define("drwav_init_file_wWidget", "Bidoodrwav_init_file_wWidget");
    build.define("drwav_init_file_with_metadata", "Bidoodrwav_init_file_with_metadata");
    build.define("modeldrwav_init_file_with_metadata", "modelBidoodrwav_init_file_with_metadata");
    build.define("drwav_init_file_with_metadataWidget", "Bidoodrwav_init_file_with_metadataWidget");
    build.define("drwav_init_file_with_metadata_w", "Bidoodrwav_init_file_with_metadata_w");
    build.define("modeldrwav_init_file_with_metadata_w", "modelBidoodrwav_init_file_with_metadata_w");
    build.define("drwav_init_file_with_metadata_wWidget", "Bidoodrwav_init_file_with_metadata_wWidget");
    build.define("drwav_init_file_write", "Bidoodrwav_init_file_write");
    build.define("modeldrwav_init_file_write", "modelBidoodrwav_init_file_write");
    build.define("drwav_init_file_writeWidget", "Bidoodrwav_init_file_writeWidget");
    build.define("drwav_init_file_write", "Bidoodrwav_init_file_write");
    build.define("modeldrwav_init_file_write", "modelBidoodrwav_init_file_write");
    build.define("drwav_init_file_writeWidget", "Bidoodrwav_init_file_writeWidget");
    build.define("drwav_init_file_write__internal", "Bidoodrwav_init_file_write__internal");
    build.define("modeldrwav_init_file_write__internal", "modelBidoodrwav_init_file_write__internal");
    build.define("drwav_init_file_write__internalWidget", "Bidoodrwav_init_file_write__internalWidget");
    build.define("drwav_init_file_write__internal", "Bidoodrwav_init_file_write__internal");
    build.define("modeldrwav_init_file_write__internal", "modelBidoodrwav_init_file_write__internal");
    build.define("drwav_init_file_write__internalWidget", "Bidoodrwav_init_file_write__internalWidget");
    build.define("drwav_init_file_write_sequential", "Bidoodrwav_init_file_write_sequential");
    build.define("modeldrwav_init_file_write_sequential", "modelBidoodrwav_init_file_write_sequential");
    build.define("drwav_init_file_write_sequentialWidget", "Bidoodrwav_init_file_write_sequentialWidget");
    build.define("drwav_init_file_write_sequential", "Bidoodrwav_init_file_write_sequential");
    build.define("modeldrwav_init_file_write_sequential", "modelBidoodrwav_init_file_write_sequential");
    build.define("drwav_init_file_write_sequentialWidget", "Bidoodrwav_init_file_write_sequentialWidget");
    build.define("drwav_init_file_write_sequential_pcm_frames", "Bidoodrwav_init_file_write_sequential_pcm_frames");
    build.define("modeldrwav_init_file_write_sequential_pcm_frames", "modelBidoodrwav_init_file_write_sequential_pcm_frames");
    build.define("drwav_init_file_write_sequential_pcm_framesWidget", "Bidoodrwav_init_file_write_sequential_pcm_framesWidget");
    build.define("drwav_init_file_write_sequential_pcm_frames_w", "Bidoodrwav_init_file_write_sequential_pcm_frames_w");
    build.define("modeldrwav_init_file_write_sequential_pcm_frames_w", "modelBidoodrwav_init_file_write_sequential_pcm_frames_w");
    build.define("drwav_init_file_write_sequential_pcm_frames_wWidget", "Bidoodrwav_init_file_write_sequential_pcm_frames_wWidget");
    build.define("drwav_init_file_write_sequential_w", "Bidoodrwav_init_file_write_sequential_w");
    build.define("modeldrwav_init_file_write_sequential_w", "modelBidoodrwav_init_file_write_sequential_w");
    build.define("drwav_init_file_write_sequential_wWidget", "Bidoodrwav_init_file_write_sequential_wWidget");
    build.define("drwav_init_file_write_w", "Bidoodrwav_init_file_write_w");
    build.define("modeldrwav_init_file_write_w", "modelBidoodrwav_init_file_write_w");
    build.define("drwav_init_file_write_wWidget", "Bidoodrwav_init_file_write_wWidget");
    build.define("drwav_init_memory", "Bidoodrwav_init_memory");
    build.define("modeldrwav_init_memory", "modelBidoodrwav_init_memory");
    build.define("drwav_init_memoryWidget", "Bidoodrwav_init_memoryWidget");
    build.define("drwav_init_memory_ex", "Bidoodrwav_init_memory_ex");
    build.define("modeldrwav_init_memory_ex", "modelBidoodrwav_init_memory_ex");
    build.define("drwav_init_memory_exWidget", "Bidoodrwav_init_memory_exWidget");
    build.define("drwav_init_memory_with_metadata", "Bidoodrwav_init_memory_with_metadata");
    build.define("modeldrwav_init_memory_with_metadata", "modelBidoodrwav_init_memory_with_metadata");
    build.define("drwav_init_memory_with_metadataWidget", "Bidoodrwav_init_memory_with_metadataWidget");
    build.define("drwav_init_memory_write", "Bidoodrwav_init_memory_write");
    build.define("modeldrwav_init_memory_write", "modelBidoodrwav_init_memory_write");
    build.define("drwav_init_memory_writeWidget", "Bidoodrwav_init_memory_writeWidget");
    build.define("drwav_init_memory_write", "Bidoodrwav_init_memory_write");
    build.define("modeldrwav_init_memory_write", "modelBidoodrwav_init_memory_write");
    build.define("drwav_init_memory_writeWidget", "Bidoodrwav_init_memory_writeWidget");
    build.define("drwav_init_memory_write__internal", "Bidoodrwav_init_memory_write__internal");
    build.define("modeldrwav_init_memory_write__internal", "modelBidoodrwav_init_memory_write__internal");
    build.define("drwav_init_memory_write__internalWidget", "Bidoodrwav_init_memory_write__internalWidget");
    build.define("drwav_init_memory_write__internal", "Bidoodrwav_init_memory_write__internal");
    build.define("modeldrwav_init_memory_write__internal", "modelBidoodrwav_init_memory_write__internal");
    build.define("drwav_init_memory_write__internalWidget", "Bidoodrwav_init_memory_write__internalWidget");
    build.define("drwav_init_memory_write_sequential", "Bidoodrwav_init_memory_write_sequential");
    build.define("modeldrwav_init_memory_write_sequential", "modelBidoodrwav_init_memory_write_sequential");
    build.define("drwav_init_memory_write_sequentialWidget", "Bidoodrwav_init_memory_write_sequentialWidget");
    build.define("drwav_init_memory_write_sequential_pcm_frames", "Bidoodrwav_init_memory_write_sequential_pcm_frames");
    build.define("modeldrwav_init_memory_write_sequential_pcm_frames", "modelBidoodrwav_init_memory_write_sequential_pcm_frames");
    build.define("drwav_init_memory_write_sequential_pcm_framesWidget", "Bidoodrwav_init_memory_write_sequential_pcm_framesWidget");
    build.define("drwav_init_with_metadata", "Bidoodrwav_init_with_metadata");
    build.define("modeldrwav_init_with_metadata", "modelBidoodrwav_init_with_metadata");
    build.define("drwav_init_with_metadataWidget", "Bidoodrwav_init_with_metadataWidget");
    build.define("drwav_init_write", "Bidoodrwav_init_write");
    build.define("modeldrwav_init_write", "modelBidoodrwav_init_write");
    build.define("drwav_init_writeWidget", "Bidoodrwav_init_writeWidget");
    build.define("drwav_init_write", "Bidoodrwav_init_write");
    build.define("modeldrwav_init_write", "modelBidoodrwav_init_write");
    build.define("drwav_init_writeWidget", "Bidoodrwav_init_writeWidget");
    build.define("drwav_init_write__internal", "Bidoodrwav_init_write__internal");
    build.define("modeldrwav_init_write__internal", "modelBidoodrwav_init_write__internal");
    build.define("drwav_init_write__internalWidget", "Bidoodrwav_init_write__internalWidget");
    build.define("drwav_init_write_sequential", "Bidoodrwav_init_write_sequential");
    build.define("modeldrwav_init_write_sequential", "modelBidoodrwav_init_write_sequential");
    build.define("drwav_init_write_sequentialWidget", "Bidoodrwav_init_write_sequentialWidget");
    build.define("drwav_init_write_sequential_pcm_frames", "Bidoodrwav_init_write_sequential_pcm_frames");
    build.define("modeldrwav_init_write_sequential_pcm_frames", "modelBidoodrwav_init_write_sequential_pcm_frames");
    build.define("drwav_init_write_sequential_pcm_framesWidget", "Bidoodrwav_init_write_sequential_pcm_framesWidget");
    build.define("drwav_init_write_with_metadata", "Bidoodrwav_init_write_with_metadata");
    build.define("modeldrwav_init_write_with_metadata", "modelBidoodrwav_init_write_with_metadata");
    build.define("drwav_init_write_with_metadataWidget", "Bidoodrwav_init_write_with_metadataWidget");
    build.define("drwav_metadata", "Bidoodrwav_metadata");
    build.define("modeldrwav_metadata", "modelBidoodrwav_metadata");
    build.define("drwav_metadataWidget", "Bidoodrwav_metadataWidget");
    build.define("drwav__metadata_parser", "Bidoodrwav__metadata_parser");
    build.define("modeldrwav__metadata_parser", "modelBidoodrwav__metadata_parser");
    build.define("drwav__metadata_parserWidget", "Bidoodrwav__metadata_parserWidget");
    build.define("drwav_mulaw_to_f32", "Bidoodrwav_mulaw_to_f32");
    build.define("modeldrwav_mulaw_to_f32", "modelBidoodrwav_mulaw_to_f32");
    build.define("drwav_mulaw_to_f32Widget", "Bidoodrwav_mulaw_to_f32Widget");
    build.define("drwav_mulaw_to_s16", "Bidoodrwav_mulaw_to_s16");
    build.define("modeldrwav_mulaw_to_s16", "modelBidoodrwav_mulaw_to_s16");
    build.define("drwav_mulaw_to_s16Widget", "Bidoodrwav_mulaw_to_s16Widget");
    build.define("drwav_mulaw_to_s16", "Bidoodrwav_mulaw_to_s16");
    build.define("modeldrwav_mulaw_to_s16", "modelBidoodrwav_mulaw_to_s16");
    build.define("drwav_mulaw_to_s16Widget", "Bidoodrwav_mulaw_to_s16Widget");
    build.define("drwav_mulaw_to_s32", "Bidoodrwav_mulaw_to_s32");
    build.define("modeldrwav_mulaw_to_s32", "modelBidoodrwav_mulaw_to_s32");
    build.define("drwav_mulaw_to_s32Widget", "Bidoodrwav_mulaw_to_s32Widget");
    build.define("drwav_open", "Bidoodrwav_open");
    build.define("modeldrwav_open", "modelBidoodrwav_open");
    build.define("drwav_openWidget", "Bidoodrwav_openWidget");
    build.define("drwav_open_and_read_f32", "Bidoodrwav_open_and_read_f32");
    build.define("modeldrwav_open_and_read_f32", "modelBidoodrwav_open_and_read_f32");
    build.define("drwav_open_and_read_f32Widget", "Bidoodrwav_open_and_read_f32Widget");
    build.define("drwav_open_and_read_file_f32", "Bidoodrwav_open_and_read_file_f32");
    build.define("modeldrwav_open_and_read_file_f32", "modelBidoodrwav_open_and_read_file_f32");
    build.define("drwav_open_and_read_file_f32Widget", "Bidoodrwav_open_and_read_file_f32Widget");
    build.define("drwav_open_and_read_file_s16", "Bidoodrwav_open_and_read_file_s16");
    build.define("modeldrwav_open_and_read_file_s16", "modelBidoodrwav_open_and_read_file_s16");
    build.define("drwav_open_and_read_file_s16Widget", "Bidoodrwav_open_and_read_file_s16Widget");
    build.define("drwav_open_and_read_file_s32", "Bidoodrwav_open_and_read_file_s32");
    build.define("modeldrwav_open_and_read_file_s32", "modelBidoodrwav_open_and_read_file_s32");
    build.define("drwav_open_and_read_file_s32Widget", "Bidoodrwav_open_and_read_file_s32Widget");
    build.define("drwav_open_and_read_memory_f32", "Bidoodrwav_open_and_read_memory_f32");
    build.define("modeldrwav_open_and_read_memory_f32", "modelBidoodrwav_open_and_read_memory_f32");
    build.define("drwav_open_and_read_memory_f32Widget", "Bidoodrwav_open_and_read_memory_f32Widget");
    build.define("drwav_open_and_read_memory_s16", "Bidoodrwav_open_and_read_memory_s16");
    build.define("modeldrwav_open_and_read_memory_s16", "modelBidoodrwav_open_and_read_memory_s16");
    build.define("drwav_open_and_read_memory_s16Widget", "Bidoodrwav_open_and_read_memory_s16Widget");
    build.define("drwav_open_and_read_memory_s32", "Bidoodrwav_open_and_read_memory_s32");
    build.define("modeldrwav_open_and_read_memory_s32", "modelBidoodrwav_open_and_read_memory_s32");
    build.define("drwav_open_and_read_memory_s32Widget", "Bidoodrwav_open_and_read_memory_s32Widget");
    build.define("drwav_open_and_read_pcm_frames_f32", "Bidoodrwav_open_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_and_read_pcm_frames_f32", "modelBidoodrwav_open_and_read_pcm_frames_f32");
    build.define("drwav_open_and_read_pcm_frames_f32Widget", "Bidoodrwav_open_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_and_read_pcm_frames_s16", "Bidoodrwav_open_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_and_read_pcm_frames_s16", "modelBidoodrwav_open_and_read_pcm_frames_s16");
    build.define("drwav_open_and_read_pcm_frames_s16Widget", "Bidoodrwav_open_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_and_read_pcm_frames_s32", "Bidoodrwav_open_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_and_read_pcm_frames_s32", "modelBidoodrwav_open_and_read_pcm_frames_s32");
    build.define("drwav_open_and_read_pcm_frames_s32Widget", "Bidoodrwav_open_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_and_read_s16", "Bidoodrwav_open_and_read_s16");
    build.define("modeldrwav_open_and_read_s16", "modelBidoodrwav_open_and_read_s16");
    build.define("drwav_open_and_read_s16Widget", "Bidoodrwav_open_and_read_s16Widget");
    build.define("drwav_open_and_read_s32", "Bidoodrwav_open_and_read_s32");
    build.define("modeldrwav_open_and_read_s32", "modelBidoodrwav_open_and_read_s32");
    build.define("drwav_open_and_read_s32Widget", "Bidoodrwav_open_and_read_s32Widget");
    build.define("drwav_open_ex", "Bidoodrwav_open_ex");
    build.define("modeldrwav_open_ex", "modelBidoodrwav_open_ex");
    build.define("drwav_open_exWidget", "Bidoodrwav_open_exWidget");
    build.define("drwav_open_file", "Bidoodrwav_open_file");
    build.define("modeldrwav_open_file", "modelBidoodrwav_open_file");
    build.define("drwav_open_fileWidget", "Bidoodrwav_open_fileWidget");
    build.define("drwav_open_file_and_read_f32", "Bidoodrwav_open_file_and_read_f32");
    build.define("modeldrwav_open_file_and_read_f32", "modelBidoodrwav_open_file_and_read_f32");
    build.define("drwav_open_file_and_read_f32Widget", "Bidoodrwav_open_file_and_read_f32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_f32", "Bidoodrwav_open_file_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_file_and_read_pcm_frames_f32", "modelBidoodrwav_open_file_and_read_pcm_frames_f32");
    build.define("drwav_open_file_and_read_pcm_frames_f32Widget", "Bidoodrwav_open_file_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_f32_w", "Bidoodrwav_open_file_and_read_pcm_frames_f32_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_f32_w", "modelBidoodrwav_open_file_and_read_pcm_frames_f32_w");
    build.define("drwav_open_file_and_read_pcm_frames_f32_wWidget", "Bidoodrwav_open_file_and_read_pcm_frames_f32_wWidget");
    build.define("drwav_open_file_and_read_pcm_frames_s16", "Bidoodrwav_open_file_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s16", "modelBidoodrwav_open_file_and_read_pcm_frames_s16");
    build.define("drwav_open_file_and_read_pcm_frames_s16Widget", "Bidoodrwav_open_file_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_file_and_read_pcm_frames_s16_w", "Bidoodrwav_open_file_and_read_pcm_frames_s16_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s16_w", "modelBidoodrwav_open_file_and_read_pcm_frames_s16_w");
    build.define("drwav_open_file_and_read_pcm_frames_s16_wWidget", "Bidoodrwav_open_file_and_read_pcm_frames_s16_wWidget");
    build.define("drwav_open_file_and_read_pcm_frames_s32", "Bidoodrwav_open_file_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s32", "modelBidoodrwav_open_file_and_read_pcm_frames_s32");
    build.define("drwav_open_file_and_read_pcm_frames_s32Widget", "Bidoodrwav_open_file_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_file_and_read_pcm_frames_s32_w", "Bidoodrwav_open_file_and_read_pcm_frames_s32_w");
    build.define("modeldrwav_open_file_and_read_pcm_frames_s32_w", "modelBidoodrwav_open_file_and_read_pcm_frames_s32_w");
    build.define("drwav_open_file_and_read_pcm_frames_s32_wWidget", "Bidoodrwav_open_file_and_read_pcm_frames_s32_wWidget");
    build.define("drwav_open_file_and_read_s16", "Bidoodrwav_open_file_and_read_s16");
    build.define("modeldrwav_open_file_and_read_s16", "modelBidoodrwav_open_file_and_read_s16");
    build.define("drwav_open_file_and_read_s16Widget", "Bidoodrwav_open_file_and_read_s16Widget");
    build.define("drwav_open_file_and_read_s32", "Bidoodrwav_open_file_and_read_s32");
    build.define("modeldrwav_open_file_and_read_s32", "modelBidoodrwav_open_file_and_read_s32");
    build.define("drwav_open_file_and_read_s32Widget", "Bidoodrwav_open_file_and_read_s32Widget");
    build.define("drwav_open_file_ex", "Bidoodrwav_open_file_ex");
    build.define("modeldrwav_open_file_ex", "modelBidoodrwav_open_file_ex");
    build.define("drwav_open_file_exWidget", "Bidoodrwav_open_file_exWidget");
    build.define("drwav_open_file_write", "Bidoodrwav_open_file_write");
    build.define("modeldrwav_open_file_write", "modelBidoodrwav_open_file_write");
    build.define("drwav_open_file_writeWidget", "Bidoodrwav_open_file_writeWidget");
    build.define("drwav_open_file_write", "Bidoodrwav_open_file_write");
    build.define("modeldrwav_open_file_write", "modelBidoodrwav_open_file_write");
    build.define("drwav_open_file_writeWidget", "Bidoodrwav_open_file_writeWidget");
    build.define("drwav_open_file_write__internal", "Bidoodrwav_open_file_write__internal");
    build.define("modeldrwav_open_file_write__internal", "modelBidoodrwav_open_file_write__internal");
    build.define("drwav_open_file_write__internalWidget", "Bidoodrwav_open_file_write__internalWidget");
    build.define("drwav_open_file_write__internal", "Bidoodrwav_open_file_write__internal");
    build.define("modeldrwav_open_file_write__internal", "modelBidoodrwav_open_file_write__internal");
    build.define("drwav_open_file_write__internalWidget", "Bidoodrwav_open_file_write__internalWidget");
    build.define("drwav_open_file_write_sequential", "Bidoodrwav_open_file_write_sequential");
    build.define("modeldrwav_open_file_write_sequential", "modelBidoodrwav_open_file_write_sequential");
    build.define("drwav_open_file_write_sequentialWidget", "Bidoodrwav_open_file_write_sequentialWidget");
    build.define("drwav_open_file_write_sequential", "Bidoodrwav_open_file_write_sequential");
    build.define("modeldrwav_open_file_write_sequential", "modelBidoodrwav_open_file_write_sequential");
    build.define("drwav_open_file_write_sequentialWidget", "Bidoodrwav_open_file_write_sequentialWidget");
    build.define("drwav_open_memory", "Bidoodrwav_open_memory");
    build.define("modeldrwav_open_memory", "modelBidoodrwav_open_memory");
    build.define("drwav_open_memoryWidget", "Bidoodrwav_open_memoryWidget");
    build.define("drwav_open_memory_and_read_f32", "Bidoodrwav_open_memory_and_read_f32");
    build.define("modeldrwav_open_memory_and_read_f32", "modelBidoodrwav_open_memory_and_read_f32");
    build.define("drwav_open_memory_and_read_f32Widget", "Bidoodrwav_open_memory_and_read_f32Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_f32", "Bidoodrwav_open_memory_and_read_pcm_frames_f32");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_f32", "modelBidoodrwav_open_memory_and_read_pcm_frames_f32");
    build.define("drwav_open_memory_and_read_pcm_frames_f32Widget", "Bidoodrwav_open_memory_and_read_pcm_frames_f32Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_s16", "Bidoodrwav_open_memory_and_read_pcm_frames_s16");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_s16", "modelBidoodrwav_open_memory_and_read_pcm_frames_s16");
    build.define("drwav_open_memory_and_read_pcm_frames_s16Widget", "Bidoodrwav_open_memory_and_read_pcm_frames_s16Widget");
    build.define("drwav_open_memory_and_read_pcm_frames_s32", "Bidoodrwav_open_memory_and_read_pcm_frames_s32");
    build.define("modeldrwav_open_memory_and_read_pcm_frames_s32", "modelBidoodrwav_open_memory_and_read_pcm_frames_s32");
    build.define("drwav_open_memory_and_read_pcm_frames_s32Widget", "Bidoodrwav_open_memory_and_read_pcm_frames_s32Widget");
    build.define("drwav_open_memory_and_read_s16", "Bidoodrwav_open_memory_and_read_s16");
    build.define("modeldrwav_open_memory_and_read_s16", "modelBidoodrwav_open_memory_and_read_s16");
    build.define("drwav_open_memory_and_read_s16Widget", "Bidoodrwav_open_memory_and_read_s16Widget");
    build.define("drwav_open_memory_and_read_s32", "Bidoodrwav_open_memory_and_read_s32");
    build.define("modeldrwav_open_memory_and_read_s32", "modelBidoodrwav_open_memory_and_read_s32");
    build.define("drwav_open_memory_and_read_s32Widget", "Bidoodrwav_open_memory_and_read_s32Widget");
    build.define("drwav_open_memory_ex", "Bidoodrwav_open_memory_ex");
    build.define("modeldrwav_open_memory_ex", "modelBidoodrwav_open_memory_ex");
    build.define("drwav_open_memory_exWidget", "Bidoodrwav_open_memory_exWidget");
    build.define("drwav_open_memory_write", "Bidoodrwav_open_memory_write");
    build.define("modeldrwav_open_memory_write", "modelBidoodrwav_open_memory_write");
    build.define("drwav_open_memory_writeWidget", "Bidoodrwav_open_memory_writeWidget");
    build.define("drwav_open_memory_write", "Bidoodrwav_open_memory_write");
    build.define("modeldrwav_open_memory_write", "modelBidoodrwav_open_memory_write");
    build.define("drwav_open_memory_writeWidget", "Bidoodrwav_open_memory_writeWidget");
    build.define("drwav_open_memory_write__internal", "Bidoodrwav_open_memory_write__internal");
    build.define("modeldrwav_open_memory_write__internal", "modelBidoodrwav_open_memory_write__internal");
    build.define("drwav_open_memory_write__internalWidget", "Bidoodrwav_open_memory_write__internalWidget");
    build.define("drwav_open_memory_write__internal", "Bidoodrwav_open_memory_write__internal");
    build.define("modeldrwav_open_memory_write__internal", "modelBidoodrwav_open_memory_write__internal");
    build.define("drwav_open_memory_write__internalWidget", "Bidoodrwav_open_memory_write__internalWidget");
    build.define("drwav_open_memory_write_sequential", "Bidoodrwav_open_memory_write_sequential");
    build.define("modeldrwav_open_memory_write_sequential", "modelBidoodrwav_open_memory_write_sequential");
    build.define("drwav_open_memory_write_sequentialWidget", "Bidoodrwav_open_memory_write_sequentialWidget");
    build.define("drwav_open_write", "Bidoodrwav_open_write");
    build.define("modeldrwav_open_write", "modelBidoodrwav_open_write");
    build.define("drwav_open_writeWidget", "Bidoodrwav_open_writeWidget");
    build.define("drwav_open_write", "Bidoodrwav_open_write");
    build.define("modeldrwav_open_write", "modelBidoodrwav_open_write");
    build.define("drwav_open_writeWidget", "Bidoodrwav_open_writeWidget");
    build.define("drwav_open_write__internal", "Bidoodrwav_open_write__internal");
    build.define("modeldrwav_open_write__internal", "modelBidoodrwav_open_write__internal");
    build.define("drwav_open_write__internalWidget", "Bidoodrwav_open_write__internalWidget");
    build.define("drwav_open_write_sequential", "Bidoodrwav_open_write_sequential");
    build.define("modeldrwav_open_write_sequential", "modelBidoodrwav_open_write_sequential");
    build.define("drwav_open_write_sequentialWidget", "Bidoodrwav_open_write_sequentialWidget");
    build.define("drwav_read", "Bidoodrwav_read");
    build.define("modeldrwav_read", "modelBidoodrwav_read");
    build.define("drwav_readWidget", "Bidoodrwav_readWidget");
    build.define("drwav_read_f32", "Bidoodrwav_read_f32");
    build.define("modeldrwav_read_f32", "modelBidoodrwav_read_f32");
    build.define("drwav_read_f32Widget", "Bidoodrwav_read_f32Widget");
    build.define("drwav_read_f32__alaw", "Bidoodrwav_read_f32__alaw");
    build.define("modeldrwav_read_f32__alaw", "modelBidoodrwav_read_f32__alaw");
    build.define("drwav_read_f32__alawWidget", "Bidoodrwav_read_f32__alawWidget");
    build.define("drwav_read_f32__alaw", "Bidoodrwav_read_f32__alaw");
    build.define("modeldrwav_read_f32__alaw", "modelBidoodrwav_read_f32__alaw");
    build.define("drwav_read_f32__alawWidget", "Bidoodrwav_read_f32__alawWidget");
    build.define("drwav_read_f32__ieee", "Bidoodrwav_read_f32__ieee");
    build.define("modeldrwav_read_f32__ieee", "modelBidoodrwav_read_f32__ieee");
    build.define("drwav_read_f32__ieeeWidget", "Bidoodrwav_read_f32__ieeeWidget");
    build.define("drwav_read_f32__ieee", "Bidoodrwav_read_f32__ieee");
    build.define("modeldrwav_read_f32__ieee", "modelBidoodrwav_read_f32__ieee");
    build.define("drwav_read_f32__ieeeWidget", "Bidoodrwav_read_f32__ieeeWidget");
    build.define("drwav_read_f32__ima", "Bidoodrwav_read_f32__ima");
    build.define("modeldrwav_read_f32__ima", "modelBidoodrwav_read_f32__ima");
    build.define("drwav_read_f32__imaWidget", "Bidoodrwav_read_f32__imaWidget");
    build.define("drwav_read_f32__ima", "Bidoodrwav_read_f32__ima");
    build.define("modeldrwav_read_f32__ima", "modelBidoodrwav_read_f32__ima");
    build.define("drwav_read_f32__imaWidget", "Bidoodrwav_read_f32__imaWidget");
    build.define("drwav_read_f32__msadpcm", "Bidoodrwav_read_f32__msadpcm");
    build.define("modeldrwav_read_f32__msadpcm", "modelBidoodrwav_read_f32__msadpcm");
    build.define("drwav_read_f32__msadpcmWidget", "Bidoodrwav_read_f32__msadpcmWidget");
    build.define("drwav_read_f32__msadpcm", "Bidoodrwav_read_f32__msadpcm");
    build.define("modeldrwav_read_f32__msadpcm", "modelBidoodrwav_read_f32__msadpcm");
    build.define("drwav_read_f32__msadpcmWidget", "Bidoodrwav_read_f32__msadpcmWidget");
    build.define("drwav_read_f32__mulaw", "Bidoodrwav_read_f32__mulaw");
    build.define("modeldrwav_read_f32__mulaw", "modelBidoodrwav_read_f32__mulaw");
    build.define("drwav_read_f32__mulawWidget", "Bidoodrwav_read_f32__mulawWidget");
    build.define("drwav_read_f32__mulaw", "Bidoodrwav_read_f32__mulaw");
    build.define("modeldrwav_read_f32__mulaw", "modelBidoodrwav_read_f32__mulaw");
    build.define("drwav_read_f32__mulawWidget", "Bidoodrwav_read_f32__mulawWidget");
    build.define("drwav_read_f32__pcm", "Bidoodrwav_read_f32__pcm");
    build.define("modeldrwav_read_f32__pcm", "modelBidoodrwav_read_f32__pcm");
    build.define("drwav_read_f32__pcmWidget", "Bidoodrwav_read_f32__pcmWidget");
    build.define("drwav_read_f32__pcm", "Bidoodrwav_read_f32__pcm");
    build.define("modeldrwav_read_f32__pcm", "modelBidoodrwav_read_f32__pcm");
    build.define("drwav_read_f32__pcmWidget", "Bidoodrwav_read_f32__pcmWidget");
    build.define("drwav_read_pcm_frames", "Bidoodrwav_read_pcm_frames");
    build.define("modeldrwav_read_pcm_frames", "modelBidoodrwav_read_pcm_frames");
    build.define("drwav_read_pcm_framesWidget", "Bidoodrwav_read_pcm_framesWidget");
    build.define("drwav_read_pcm_frames_be", "Bidoodrwav_read_pcm_frames_be");
    build.define("modeldrwav_read_pcm_frames_be", "modelBidoodrwav_read_pcm_frames_be");
    build.define("drwav_read_pcm_frames_beWidget", "Bidoodrwav_read_pcm_frames_beWidget");
    build.define("drwav_read_pcm_frames_f32", "Bidoodrwav_read_pcm_frames_f32");
    build.define("modeldrwav_read_pcm_frames_f32", "modelBidoodrwav_read_pcm_frames_f32");
    build.define("drwav_read_pcm_frames_f32Widget", "Bidoodrwav_read_pcm_frames_f32Widget");
    build.define("drwav_read_pcm_frames_f32be", "Bidoodrwav_read_pcm_frames_f32be");
    build.define("modeldrwav_read_pcm_frames_f32be", "modelBidoodrwav_read_pcm_frames_f32be");
    build.define("drwav_read_pcm_frames_f32beWidget", "Bidoodrwav_read_pcm_frames_f32beWidget");
    build.define("drwav_read_pcm_frames_f32le", "Bidoodrwav_read_pcm_frames_f32le");
    build.define("modeldrwav_read_pcm_frames_f32le", "modelBidoodrwav_read_pcm_frames_f32le");
    build.define("drwav_read_pcm_frames_f32leWidget", "Bidoodrwav_read_pcm_frames_f32leWidget");
    build.define("drwav_read_pcm_frames_le", "Bidoodrwav_read_pcm_frames_le");
    build.define("modeldrwav_read_pcm_frames_le", "modelBidoodrwav_read_pcm_frames_le");
    build.define("drwav_read_pcm_frames_leWidget", "Bidoodrwav_read_pcm_frames_leWidget");
    build.define("drwav_read_pcm_frames_s16", "Bidoodrwav_read_pcm_frames_s16");
    build.define("modeldrwav_read_pcm_frames_s16", "modelBidoodrwav_read_pcm_frames_s16");
    build.define("drwav_read_pcm_frames_s16Widget", "Bidoodrwav_read_pcm_frames_s16Widget");
    build.define("drwav_read_pcm_frames_s16be", "Bidoodrwav_read_pcm_frames_s16be");
    build.define("modeldrwav_read_pcm_frames_s16be", "modelBidoodrwav_read_pcm_frames_s16be");
    build.define("drwav_read_pcm_frames_s16beWidget", "Bidoodrwav_read_pcm_frames_s16beWidget");
    build.define("drwav_read_pcm_frames_s16le", "Bidoodrwav_read_pcm_frames_s16le");
    build.define("modeldrwav_read_pcm_frames_s16le", "modelBidoodrwav_read_pcm_frames_s16le");
    build.define("drwav_read_pcm_frames_s16leWidget", "Bidoodrwav_read_pcm_frames_s16leWidget");
    build.define("drwav_read_pcm_frames_s32", "Bidoodrwav_read_pcm_frames_s32");
    build.define("modeldrwav_read_pcm_frames_s32", "modelBidoodrwav_read_pcm_frames_s32");
    build.define("drwav_read_pcm_frames_s32Widget", "Bidoodrwav_read_pcm_frames_s32Widget");
    build.define("drwav_read_pcm_frames_s32be", "Bidoodrwav_read_pcm_frames_s32be");
    build.define("modeldrwav_read_pcm_frames_s32be", "modelBidoodrwav_read_pcm_frames_s32be");
    build.define("drwav_read_pcm_frames_s32beWidget", "Bidoodrwav_read_pcm_frames_s32beWidget");
    build.define("drwav_read_pcm_frames_s32le", "Bidoodrwav_read_pcm_frames_s32le");
    build.define("modeldrwav_read_pcm_frames_s32le", "modelBidoodrwav_read_pcm_frames_s32le");
    build.define("drwav_read_pcm_frames_s32leWidget", "Bidoodrwav_read_pcm_frames_s32leWidget");
    build.define("drwav_read_raw", "Bidoodrwav_read_raw");
    build.define("modeldrwav_read_raw", "modelBidoodrwav_read_raw");
    build.define("drwav_read_rawWidget", "Bidoodrwav_read_rawWidget");
    build.define("drwav_read_s16", "Bidoodrwav_read_s16");
    build.define("modeldrwav_read_s16", "modelBidoodrwav_read_s16");
    build.define("drwav_read_s16Widget", "Bidoodrwav_read_s16Widget");
    build.define("drwav_read_s16__alaw", "Bidoodrwav_read_s16__alaw");
    build.define("modeldrwav_read_s16__alaw", "modelBidoodrwav_read_s16__alaw");
    build.define("drwav_read_s16__alawWidget", "Bidoodrwav_read_s16__alawWidget");
    build.define("drwav_read_s16__ieee", "Bidoodrwav_read_s16__ieee");
    build.define("modeldrwav_read_s16__ieee", "modelBidoodrwav_read_s16__ieee");
    build.define("drwav_read_s16__ieeeWidget", "Bidoodrwav_read_s16__ieeeWidget");
    build.define("drwav_read_s16__ima", "Bidoodrwav_read_s16__ima");
    build.define("modeldrwav_read_s16__ima", "modelBidoodrwav_read_s16__ima");
    build.define("drwav_read_s16__imaWidget", "Bidoodrwav_read_s16__imaWidget");
    build.define("drwav_read_s16__msadpcm", "Bidoodrwav_read_s16__msadpcm");
    build.define("modeldrwav_read_s16__msadpcm", "modelBidoodrwav_read_s16__msadpcm");
    build.define("drwav_read_s16__msadpcmWidget", "Bidoodrwav_read_s16__msadpcmWidget");
    build.define("drwav_read_s16__mulaw", "Bidoodrwav_read_s16__mulaw");
    build.define("modeldrwav_read_s16__mulaw", "modelBidoodrwav_read_s16__mulaw");
    build.define("drwav_read_s16__mulawWidget", "Bidoodrwav_read_s16__mulawWidget");
    build.define("drwav_read_s16__pcm", "Bidoodrwav_read_s16__pcm");
    build.define("modeldrwav_read_s16__pcm", "modelBidoodrwav_read_s16__pcm");
    build.define("drwav_read_s16__pcmWidget", "Bidoodrwav_read_s16__pcmWidget");
    build.define("drwav_read_s32", "Bidoodrwav_read_s32");
    build.define("modeldrwav_read_s32", "modelBidoodrwav_read_s32");
    build.define("drwav_read_s32Widget", "Bidoodrwav_read_s32Widget");
    build.define("drwav_read_s32__alaw", "Bidoodrwav_read_s32__alaw");
    build.define("modeldrwav_read_s32__alaw", "modelBidoodrwav_read_s32__alaw");
    build.define("drwav_read_s32__alawWidget", "Bidoodrwav_read_s32__alawWidget");
    build.define("drwav_read_s32__alaw", "Bidoodrwav_read_s32__alaw");
    build.define("modeldrwav_read_s32__alaw", "modelBidoodrwav_read_s32__alaw");
    build.define("drwav_read_s32__alawWidget", "Bidoodrwav_read_s32__alawWidget");
    build.define("drwav_read_s32__ieee", "Bidoodrwav_read_s32__ieee");
    build.define("modeldrwav_read_s32__ieee", "modelBidoodrwav_read_s32__ieee");
    build.define("drwav_read_s32__ieeeWidget", "Bidoodrwav_read_s32__ieeeWidget");
    build.define("drwav_read_s32__ieee", "Bidoodrwav_read_s32__ieee");
    build.define("modeldrwav_read_s32__ieee", "modelBidoodrwav_read_s32__ieee");
    build.define("drwav_read_s32__ieeeWidget", "Bidoodrwav_read_s32__ieeeWidget");
    build.define("drwav_read_s32__ima", "Bidoodrwav_read_s32__ima");
    build.define("modeldrwav_read_s32__ima", "modelBidoodrwav_read_s32__ima");
    build.define("drwav_read_s32__imaWidget", "Bidoodrwav_read_s32__imaWidget");
    build.define("drwav_read_s32__ima", "Bidoodrwav_read_s32__ima");
    build.define("modeldrwav_read_s32__ima", "modelBidoodrwav_read_s32__ima");
    build.define("drwav_read_s32__imaWidget", "Bidoodrwav_read_s32__imaWidget");
    build.define("drwav_read_s32__msadpcm", "Bidoodrwav_read_s32__msadpcm");
    build.define("modeldrwav_read_s32__msadpcm", "modelBidoodrwav_read_s32__msadpcm");
    build.define("drwav_read_s32__msadpcmWidget", "Bidoodrwav_read_s32__msadpcmWidget");
    build.define("drwav_read_s32__msadpcm", "Bidoodrwav_read_s32__msadpcm");
    build.define("modeldrwav_read_s32__msadpcm", "modelBidoodrwav_read_s32__msadpcm");
    build.define("drwav_read_s32__msadpcmWidget", "Bidoodrwav_read_s32__msadpcmWidget");
    build.define("drwav_read_s32__mulaw", "Bidoodrwav_read_s32__mulaw");
    build.define("modeldrwav_read_s32__mulaw", "modelBidoodrwav_read_s32__mulaw");
    build.define("drwav_read_s32__mulawWidget", "Bidoodrwav_read_s32__mulawWidget");
    build.define("drwav_read_s32__mulaw", "Bidoodrwav_read_s32__mulaw");
    build.define("modeldrwav_read_s32__mulaw", "modelBidoodrwav_read_s32__mulaw");
    build.define("drwav_read_s32__mulawWidget", "Bidoodrwav_read_s32__mulawWidget");
    build.define("drwav_read_s32__pcm", "Bidoodrwav_read_s32__pcm");
    build.define("modeldrwav_read_s32__pcm", "modelBidoodrwav_read_s32__pcm");
    build.define("drwav_read_s32__pcmWidget", "Bidoodrwav_read_s32__pcmWidget");
    build.define("drwav_read_s32__pcm", "Bidoodrwav_read_s32__pcm");
    build.define("modeldrwav_read_s32__pcm", "modelBidoodrwav_read_s32__pcm");
    build.define("drwav_read_s32__pcmWidget", "Bidoodrwav_read_s32__pcmWidget");
    build.define("drwav_riff_chunk_size_riff", "Bidoodrwav_riff_chunk_size_riff");
    build.define("modeldrwav_riff_chunk_size_riff", "modelBidoodrwav_riff_chunk_size_riff");
    build.define("drwav_riff_chunk_size_riffWidget", "Bidoodrwav_riff_chunk_size_riffWidget");
    build.define("drwav_riff_chunk_size_w64", "Bidoodrwav_riff_chunk_size_w64");
    build.define("modeldrwav_riff_chunk_size_w64", "modelBidoodrwav_riff_chunk_size_w64");
    build.define("drwav_riff_chunk_size_w64Widget", "Bidoodrwav_riff_chunk_size_w64Widget");
    build.define("drwav_s16_to_f32", "Bidoodrwav_s16_to_f32");
    build.define("modeldrwav_s16_to_f32", "modelBidoodrwav_s16_to_f32");
    build.define("drwav_s16_to_f32Widget", "Bidoodrwav_s16_to_f32Widget");
    build.define("drwav_s16_to_s32", "Bidoodrwav_s16_to_s32");
    build.define("modeldrwav_s16_to_s32", "modelBidoodrwav_s16_to_s32");
    build.define("drwav_s16_to_s32Widget", "Bidoodrwav_s16_to_s32Widget");
    build.define("drwav_s24_to_f32", "Bidoodrwav_s24_to_f32");
    build.define("modeldrwav_s24_to_f32", "modelBidoodrwav_s24_to_f32");
    build.define("drwav_s24_to_f32Widget", "Bidoodrwav_s24_to_f32Widget");
    build.define("drwav_s24_to_s16", "Bidoodrwav_s24_to_s16");
    build.define("modeldrwav_s24_to_s16", "modelBidoodrwav_s24_to_s16");
    build.define("drwav_s24_to_s16Widget", "Bidoodrwav_s24_to_s16Widget");
    build.define("drwav_s24_to_s16", "Bidoodrwav_s24_to_s16");
    build.define("modeldrwav_s24_to_s16", "modelBidoodrwav_s24_to_s16");
    build.define("drwav_s24_to_s16Widget", "Bidoodrwav_s24_to_s16Widget");
    build.define("drwav_s24_to_s32", "Bidoodrwav_s24_to_s32");
    build.define("modeldrwav_s24_to_s32", "modelBidoodrwav_s24_to_s32");
    build.define("drwav_s24_to_s32Widget", "Bidoodrwav_s24_to_s32Widget");
    build.define("drwav_s32_to_f32", "Bidoodrwav_s32_to_f32");
    build.define("modeldrwav_s32_to_f32", "modelBidoodrwav_s32_to_f32");
    build.define("drwav_s32_to_f32Widget", "Bidoodrwav_s32_to_f32Widget");
    build.define("drwav_s32_to_s16", "Bidoodrwav_s32_to_s16");
    build.define("modeldrwav_s32_to_s16", "modelBidoodrwav_s32_to_s16");
    build.define("drwav_s32_to_s16Widget", "Bidoodrwav_s32_to_s16Widget");
    build.define("drwav_s32_to_s16", "Bidoodrwav_s32_to_s16");
    build.define("modeldrwav_s32_to_s16", "modelBidoodrwav_s32_to_s16");
    build.define("drwav_s32_to_s16Widget", "Bidoodrwav_s32_to_s16Widget");
    build.define("drwav_seek_to_pcm_frame", "Bidoodrwav_seek_to_pcm_frame");
    build.define("modeldrwav_seek_to_pcm_frame", "modelBidoodrwav_seek_to_pcm_frame");
    build.define("drwav_seek_to_pcm_frameWidget", "Bidoodrwav_seek_to_pcm_frameWidget");
    build.define("drwav_seek_to_sample", "Bidoodrwav_seek_to_sample");
    build.define("modeldrwav_seek_to_sample", "modelBidoodrwav_seek_to_sample");
    build.define("drwav_seek_to_sampleWidget", "Bidoodrwav_seek_to_sampleWidget");
    build.define("drwav_seek_to_sample", "Bidoodrwav_seek_to_sample");
    build.define("modeldrwav_seek_to_sample", "modelBidoodrwav_seek_to_sample");
    build.define("drwav_seek_to_sampleWidget", "Bidoodrwav_seek_to_sampleWidget");
    build.define("drwav_smpl", "Bidoodrwav_smpl");
    build.define("modeldrwav_smpl", "modelBidoodrwav_smpl");
    build.define("drwav_smplWidget", "Bidoodrwav_smplWidget");
    build.define("drwav_smpl_loop", "Bidoodrwav_smpl_loop");
    build.define("modeldrwav_smpl_loop", "modelBidoodrwav_smpl_loop");
    build.define("drwav_smpl_loopWidget", "Bidoodrwav_smpl_loopWidget");
    build.define("drwav_take_ownership_of_metadata", "Bidoodrwav_take_ownership_of_metadata");
    build.define("modeldrwav_take_ownership_of_metadata", "modelBidoodrwav_take_ownership_of_metadata");
    build.define("drwav_take_ownership_of_metadataWidget", "Bidoodrwav_take_ownership_of_metadataWidget");
    build.define("drwav_target_write_size_bytes", "Bidoodrwav_target_write_size_bytes");
    build.define("modeldrwav_target_write_size_bytes", "modelBidoodrwav_target_write_size_bytes");
    build.define("drwav_target_write_size_bytesWidget", "Bidoodrwav_target_write_size_bytesWidget");
    build.define("drwav_u8_to_f32", "Bidoodrwav_u8_to_f32");
    build.define("modeldrwav_u8_to_f32", "modelBidoodrwav_u8_to_f32");
    build.define("drwav_u8_to_f32Widget", "Bidoodrwav_u8_to_f32Widget");
    build.define("drwav_u8_to_s16", "Bidoodrwav_u8_to_s16");
    build.define("modeldrwav_u8_to_s16", "modelBidoodrwav_u8_to_s16");
    build.define("drwav_u8_to_s16Widget", "Bidoodrwav_u8_to_s16Widget");
    build.define("drwav_u8_to_s16", "Bidoodrwav_u8_to_s16");
    build.define("modeldrwav_u8_to_s16", "modelBidoodrwav_u8_to_s16");
    build.define("drwav_u8_to_s16Widget", "Bidoodrwav_u8_to_s16Widget");
    build.define("drwav_u8_to_s32", "Bidoodrwav_u8_to_s32");
    build.define("modeldrwav_u8_to_s32", "modelBidoodrwav_u8_to_s32");
    build.define("drwav_u8_to_s32Widget", "Bidoodrwav_u8_to_s32Widget");
    build.define("drwav_uninit", "Bidoodrwav_uninit");
    build.define("modeldrwav_uninit", "modelBidoodrwav_uninit");
    build.define("drwav_uninitWidget", "Bidoodrwav_uninitWidget");
    build.define("drwav_version", "Bidoodrwav_version");
    build.define("modeldrwav_version", "modelBidoodrwav_version");
    build.define("drwav_versionWidget", "Bidoodrwav_versionWidget");
    build.define("drwav_version_string", "Bidoodrwav_version_string");
    build.define("modeldrwav_version_string", "modelBidoodrwav_version_string");
    build.define("drwav_version_stringWidget", "Bidoodrwav_version_stringWidget");
    build.define("drwav_write", "Bidoodrwav_write");
    build.define("modeldrwav_write", "modelBidoodrwav_write");
    build.define("drwav_writeWidget", "Bidoodrwav_writeWidget");
    build.define("drwav_write", "Bidoodrwav_write");
    build.define("modeldrwav_write", "modelBidoodrwav_write");
    build.define("drwav_writeWidget", "Bidoodrwav_writeWidget");
    build.define("drwav_write_pcm_frames", "Bidoodrwav_write_pcm_frames");
    build.define("modeldrwav_write_pcm_frames", "modelBidoodrwav_write_pcm_frames");
    build.define("drwav_write_pcm_framesWidget", "Bidoodrwav_write_pcm_framesWidget");
    build.define("drwav_write_pcm_frames_be", "Bidoodrwav_write_pcm_frames_be");
    build.define("modeldrwav_write_pcm_frames_be", "modelBidoodrwav_write_pcm_frames_be");
    build.define("drwav_write_pcm_frames_beWidget", "Bidoodrwav_write_pcm_frames_beWidget");
    build.define("drwav_write_pcm_frames_le", "Bidoodrwav_write_pcm_frames_le");
    build.define("modeldrwav_write_pcm_frames_le", "modelBidoodrwav_write_pcm_frames_le");
    build.define("drwav_write_pcm_frames_leWidget", "Bidoodrwav_write_pcm_frames_leWidget");
    build.define("drwav_write_raw", "Bidoodrwav_write_raw");
    build.define("modeldrwav_write_raw", "modelBidoodrwav_write_raw");
    build.define("drwav_write_rawWidget", "Bidoodrwav_write_rawWidget");

    // Filter-out list
    let filter_out: Vec<String> = vec![
        "Bidoo/src/plugin.cpp".to_string(),
        "Bidoo/src/ANTN.cpp".to_string(),
        "Bidoo/src/dep/lodepng/pngdetail.cpp".to_string(),
        "Bidoo/src/dep/resampler/main.cpp".to_string(),
    ];

    // Source files

    // Glob Bidoo/src/**/*.cpp|cc|c (recursive)
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
    collect_sources(&plugins_dir.join("Bidoo/src"), &filter_out, &plugins_dir, &mut build, 0);

    // Glob Bidoo/src/dep/**/*.cpp|cc|c (recursive)
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
    collect_sources(&plugins_dir.join("Bidoo/src/dep"), &filter_out, &plugins_dir, &mut build, 0);

    // Glob Bidoo/src/dep/filters/**/*.cpp|cc|c (recursive)
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
    collect_sources(&plugins_dir.join("Bidoo/src/dep/filters"), &filter_out, &plugins_dir, &mut build, 0);

    // Glob Bidoo/src/dep/freeverb/**/*.cpp|cc|c (recursive)
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
    collect_sources(&plugins_dir.join("Bidoo/src/dep/freeverb"), &filter_out, &plugins_dir, &mut build, 0);

    // Glob Bidoo/src/dep/lodepng/**/*.cpp|cc|c (recursive)
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
    collect_sources(&plugins_dir.join("Bidoo/src/dep/lodepng"), &filter_out, &plugins_dir, &mut build, 0);

    // Glob Bidoo/src/dep/resampler/**/*.cpp|cc|c (recursive)
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
    collect_sources(&plugins_dir.join("Bidoo/src/dep/resampler"), &filter_out, &plugins_dir, &mut build, 0);

    build.compile("cardinal_plugin_bidoo");
}
