/*
 * Simple test modules for the Cardinal Rust bridge.
 * These demonstrate the Module API without any external dependencies.
 */

#include <engine/Module.hpp>
#include <engine/Engine.hpp>
#include <plugin/Model.hpp>
#include <plugin/Plugin.hpp>
#include <plugin.hpp>
#include <context.hpp>
#include <math.hpp>
#include <dsp/common.hpp>
#include <cmath>

namespace test_modules {

using namespace rack;
using namespace rack::engine;
using rack::math::clamp;
static constexpr float FREQ_C4 = 261.6256f;

// ── VCO: Voltage Controlled Oscillator ───────────────────────────────
struct VCO : Module {
    enum ParamIds {
        FREQ_PARAM,
        FM_PARAM,
        PW_PARAM,
        NUM_PARAMS
    };
    enum InputIds {
        VOCT_INPUT,
        FM_INPUT,
        PW_INPUT,
        SYNC_INPUT,
        NUM_INPUTS
    };
    enum OutputIds {
        SIN_OUTPUT,
        TRI_OUTPUT,
        SAW_OUTPUT,
        SQR_OUTPUT,
        NUM_OUTPUTS
    };
    enum LightIds {
        NUM_LIGHTS
    };

    float phase = 0.f;

    VCO() {
        config(NUM_PARAMS, NUM_INPUTS, NUM_OUTPUTS, NUM_LIGHTS);
        configParam(FREQ_PARAM, -4.f, 4.f, 0.f, "Frequency", " Hz", 2.f, FREQ_C4);
        configParam(FM_PARAM, -1.f, 1.f, 0.f, "FM", "%", 0.f, 100.f);
        configParam(PW_PARAM, 0.01f, 0.99f, 0.5f, "Pulse width", "%", 0.f, 100.f);
        configInput(VOCT_INPUT, "V/Oct");
        configInput(FM_INPUT, "FM");
        configInput(PW_INPUT, "Pulse width");
        configInput(SYNC_INPUT, "Sync");
        configOutput(SIN_OUTPUT, "Sine");
        configOutput(TRI_OUTPUT, "Triangle");
        configOutput(SAW_OUTPUT, "Saw");
        configOutput(SQR_OUTPUT, "Square");
    }

    void process(const ProcessArgs& args) override {
        float pitch = params[FREQ_PARAM].getValue();
        pitch += inputs[VOCT_INPUT].getVoltage();
        pitch += inputs[FM_INPUT].getVoltage() * params[FM_PARAM].getValue();
        float freq = FREQ_C4 * std::pow(2.f, pitch);
        freq = std::fmax(freq, 0.f);

        phase += freq * args.sampleTime;
        if (phase >= 1.f) phase -= 1.f;

        float pw = params[PW_PARAM].getValue();
        pw += inputs[PW_INPUT].getVoltage() / 10.f;
        pw = clamp(pw, 0.01f, 0.99f);

        // Sine
        if (outputs[SIN_OUTPUT].isConnected())
            outputs[SIN_OUTPUT].setVoltage(5.f * std::sin(2.f * M_PI * phase));
        // Triangle
        if (outputs[TRI_OUTPUT].isConnected())
            outputs[TRI_OUTPUT].setVoltage(5.f * (4.f * std::fabs(phase - 0.5f) - 1.f));
        // Saw
        if (outputs[SAW_OUTPUT].isConnected())
            outputs[SAW_OUTPUT].setVoltage(5.f * (2.f * phase - 1.f));
        // Square
        if (outputs[SQR_OUTPUT].isConnected())
            outputs[SQR_OUTPUT].setVoltage(phase < pw ? 5.f : -5.f);
    }
};

// ── VCA: Voltage Controlled Amplifier ────────────────────────────────
struct VCA : Module {
    enum ParamIds {
        LEVEL_PARAM,
        NUM_PARAMS
    };
    enum InputIds {
        IN_INPUT,
        CV_INPUT,
        NUM_INPUTS
    };
    enum OutputIds {
        OUT_OUTPUT,
        NUM_OUTPUTS
    };
    enum LightIds {
        NUM_LIGHTS
    };

    VCA() {
        config(NUM_PARAMS, NUM_INPUTS, NUM_OUTPUTS, NUM_LIGHTS);
        configParam(LEVEL_PARAM, 0.f, 1.f, 1.f, "Level", "%", 0.f, 100.f);
        configInput(IN_INPUT, "Audio");
        configInput(CV_INPUT, "CV");
        configOutput(OUT_OUTPUT, "Audio");
    }

    void process(const ProcessArgs& args) override {
        float level = params[LEVEL_PARAM].getValue();
        if (inputs[CV_INPUT].isConnected())
            level *= clamp(inputs[CV_INPUT].getVoltage() / 10.f, 0.f, 1.f);
        outputs[OUT_OUTPUT].setVoltage(inputs[IN_INPUT].getVoltage() * level);
    }
};

// ── Mixer: 4-channel mixer ───────────────────────────────────────────
struct Mixer : Module {
    enum ParamIds {
        LEVEL1_PARAM,
        LEVEL2_PARAM,
        LEVEL3_PARAM,
        LEVEL4_PARAM,
        MASTER_PARAM,
        NUM_PARAMS
    };
    enum InputIds {
        IN1_INPUT,
        IN2_INPUT,
        IN3_INPUT,
        IN4_INPUT,
        NUM_INPUTS
    };
    enum OutputIds {
        MIX_OUTPUT,
        NUM_OUTPUTS
    };
    enum LightIds {
        NUM_LIGHTS
    };

    Mixer() {
        config(NUM_PARAMS, NUM_INPUTS, NUM_OUTPUTS, NUM_LIGHTS);
        configParam(LEVEL1_PARAM, 0.f, 1.f, 0.5f, "Ch 1 Level", "%", 0.f, 100.f);
        configParam(LEVEL2_PARAM, 0.f, 1.f, 0.5f, "Ch 2 Level", "%", 0.f, 100.f);
        configParam(LEVEL3_PARAM, 0.f, 1.f, 0.5f, "Ch 3 Level", "%", 0.f, 100.f);
        configParam(LEVEL4_PARAM, 0.f, 1.f, 0.5f, "Ch 4 Level", "%", 0.f, 100.f);
        configParam(MASTER_PARAM, 0.f, 1.f, 1.f, "Master", "%", 0.f, 100.f);
        configInput(IN1_INPUT, "Channel 1");
        configInput(IN2_INPUT, "Channel 2");
        configInput(IN3_INPUT, "Channel 3");
        configInput(IN4_INPUT, "Channel 4");
        configOutput(MIX_OUTPUT, "Mix");
    }

    void process(const ProcessArgs& args) override {
        float mix = 0.f;
        mix += inputs[IN1_INPUT].getVoltage() * params[LEVEL1_PARAM].getValue();
        mix += inputs[IN2_INPUT].getVoltage() * params[LEVEL2_PARAM].getValue();
        mix += inputs[IN3_INPUT].getVoltage() * params[LEVEL3_PARAM].getValue();
        mix += inputs[IN4_INPUT].getVoltage() * params[LEVEL4_PARAM].getValue();
        mix *= params[MASTER_PARAM].getValue();
        outputs[MIX_OUTPUT].setVoltage(clamp(mix, -12.f, 12.f));
    }
};

// ── LFO: Low Frequency Oscillator ────────────────────────────────────
struct LFO : Module {
    enum ParamIds {
        FREQ_PARAM,
        SHAPE_PARAM,
        NUM_PARAMS
    };
    enum InputIds {
        NUM_INPUTS
    };
    enum OutputIds {
        SIN_OUTPUT,
        TRI_OUTPUT,
        SAW_OUTPUT,
        SQR_OUTPUT,
        NUM_OUTPUTS
    };
    enum LightIds {
        PHASE_LIGHT,
        NUM_LIGHTS
    };

    float phase = 0.f;

    LFO() {
        config(NUM_PARAMS, NUM_INPUTS, NUM_OUTPUTS, NUM_LIGHTS);
        configParam(FREQ_PARAM, -8.f, 4.f, -1.f, "Frequency", " Hz", 2.f, 1.f);
        configParam(SHAPE_PARAM, 0.f, 1.f, 0.5f, "Shape", "%", 0.f, 100.f);
        configOutput(SIN_OUTPUT, "Sine");
        configOutput(TRI_OUTPUT, "Triangle");
        configOutput(SAW_OUTPUT, "Saw");
        configOutput(SQR_OUTPUT, "Square");
    }

    void process(const ProcessArgs& args) override {
        float freq = std::pow(2.f, params[FREQ_PARAM].getValue());
        phase += freq * args.sampleTime;
        if (phase >= 1.f) phase -= 1.f;

        if (outputs[SIN_OUTPUT].isConnected())
            outputs[SIN_OUTPUT].setVoltage(5.f * std::sin(2.f * M_PI * phase));
        if (outputs[TRI_OUTPUT].isConnected())
            outputs[TRI_OUTPUT].setVoltage(5.f * (4.f * std::fabs(phase - 0.5f) - 1.f));
        if (outputs[SAW_OUTPUT].isConnected())
            outputs[SAW_OUTPUT].setVoltage(5.f * (2.f * phase - 1.f));
        if (outputs[SQR_OUTPUT].isConnected())
            outputs[SQR_OUTPUT].setVoltage(phase < 0.5f ? 5.f : -5.f);

        lights[PHASE_LIGHT].setBrightness(std::sin(2.f * M_PI * phase) * 0.5f + 0.5f);
    }
};

// ── Scope: simple scope that records recent output voltage ───────────
struct Scope : Module {
    enum ParamIds {
        TIME_PARAM,
        TRIG_PARAM,
        NUM_PARAMS
    };
    enum InputIds {
        IN_INPUT,
        TRIG_INPUT,
        NUM_INPUTS
    };
    enum OutputIds {
        NUM_OUTPUTS
    };
    enum LightIds {
        NUM_LIGHTS
    };

    static const int BUFFER_SIZE = 256;
    float buffer[BUFFER_SIZE] = {};
    int bufferIndex = 0;

    Scope() {
        config(NUM_PARAMS, NUM_INPUTS, NUM_OUTPUTS, NUM_LIGHTS);
        configParam(TIME_PARAM, -6.f, -1.f, -3.f, "Time", " ms/div");
        configParam(TRIG_PARAM, -10.f, 10.f, 0.f, "Trigger level", " V");
        configInput(IN_INPUT, "Signal");
        configInput(TRIG_INPUT, "Trigger");
    }

    void process(const ProcessArgs& args) override {
        buffer[bufferIndex] = inputs[IN_INPUT].getVoltage();
        bufferIndex = (bufferIndex + 1) % BUFFER_SIZE;
    }
};

// ── Model factories ──────────────────────────────────────────────────

// Simple Model subclass that creates modules of type T
template <typename T>
struct TestModel : plugin::Model {
    engine::Module* createModule() override {
        return new T();
    }
    // We don't need widget creation for headless mode
    app::ModuleWidget* createModuleWidget(engine::Module*) override {
        return nullptr;
    }
};

static plugin::Plugin testPlugin;
static TestModel<VCO> vcoModel;
static TestModel<VCA> vcaModel;
static TestModel<Mixer> mixerModel;
static TestModel<LFO> lfoModel;
static TestModel<Scope> scopeModel;

void registerTestModules() {
    static bool registered = false;
    if (registered) return;
    registered = true;

    testPlugin.slug = "TestSuite";
    testPlugin.name = "Test Suite";
    testPlugin.version = "1.0.0";

    vcoModel.slug = "VCO";
    vcoModel.name = "VCO";
    // addModel sets model->plugin, so don't set it beforehand
    testPlugin.addModel(&vcoModel);

    vcaModel.slug = "VCA";
    vcaModel.name = "VCA";
    testPlugin.addModel(&vcaModel);

    mixerModel.slug = "Mixer";
    mixerModel.name = "Mixer";
    testPlugin.addModel(&mixerModel);

    lfoModel.slug = "LFO";
    lfoModel.name = "LFO";
    testPlugin.addModel(&lfoModel);

    scopeModel.slug = "Scope";
    scopeModel.name = "Scope";
    testPlugin.addModel(&scopeModel);

    rack::plugin::plugins.push_back(&testPlugin);
}

} // namespace test_modules
