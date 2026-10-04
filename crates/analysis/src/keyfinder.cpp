#include "keyfinder.h"
#include <cstddef>
#include <limits>

struct Analyzer {
    KeyFinder::KeyFinder finder;
    KeyFinder::Workspace workspace;
};

extern "C" {
Analyzer* openbox_key_new() noexcept {
    try { return new Analyzer(); } catch (...) { return nullptr; }
}

bool openbox_key_push(Analyzer* analyzer, const float* samples, std::size_t length) noexcept {
    try {
        if (length > std::numeric_limits<unsigned int>::max()) return false;
        KeyFinder::AudioData audio;
        audio.setFrameRate(22050);
        audio.setChannels(1);
        audio.addToSampleCount(static_cast<unsigned int>(length));
        for (std::size_t i = 0; i < length; ++i) audio.setSample(i, samples[i]);
        analyzer->finder.progressiveChromagram(audio, analyzer->workspace);
        return true;
    } catch (...) { return false; }
}

int openbox_key_finish(Analyzer* analyzer) noexcept {
    try {
        analyzer->finder.finalChromagram(analyzer->workspace);
        return static_cast<int>(analyzer->finder.keyOfChromagram(analyzer->workspace));
    } catch (...) { return -1; }
}

void openbox_key_delete(Analyzer* analyzer) noexcept { delete analyzer; }
}
