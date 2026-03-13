#include <iostream>
#include <fcntl.h>
#include <unistd.h>
#include <cstring>
#include <linux/input.h>
#include <linux/uinput.h>
#include <vector>
#include <fftw3.h>
#include <string>
#include "constants.h"

namespace SMC = StableMotionConstants;

class STFTStableMotion {
private:
    std::vector<int> buffer;
    fftw_complex *in, *out;
    fftw_plan p_forward, p_backward;

    double previous_output = 0.0;
    double alpha;
    double fractional_remainder = 0.0;

public:
    STFTStableMotion(double smoothness_level) {
        alpha = smoothness_level;

        in = (fftw_complex*) fftw_malloc(sizeof(fftw_complex) * SMC::WINDOW_SIZE);
        out = (fftw_complex*) fftw_malloc(sizeof(fftw_complex) * SMC::WINDOW_SIZE);

        p_forward = fftw_plan_dft_1d(SMC::WINDOW_SIZE, in, out, FFTW_FORWARD, FFTW_MEASURE);
        p_backward = fftw_plan_dft_1d(SMC::WINDOW_SIZE, out, in, FFTW_BACKWARD, FFTW_MEASURE);
    }

    ~STFTStableMotion() {
        fftw_destroy_plan(p_forward);
        fftw_destroy_plan(p_backward);
        fftw_free(in);
        fftw_free(out);
    }

    int apply(int new_val) {
        buffer.push_back(new_val);
        if (buffer.size() > SMC::WINDOW_SIZE) buffer.erase(buffer.begin());
        if (buffer.size() < SMC::WINDOW_SIZE) return new_val;

        for (int i = 0; i < SMC::WINDOW_SIZE; i++) {
            in[i][0] = static_cast<double>(buffer[i]);
            in[i][1] = 0.0;
        }

        fftw_execute(p_forward);

        double df = SMC::SAMPLE_RATE / SMC::WINDOW_SIZE;
        for (int i = 0; i < SMC::WINDOW_SIZE; i++) {
            double freq = (i <= SMC::WINDOW_SIZE / 2) ? (i * df) : ((SMC::WINDOW_SIZE - i) * df);
            if (freq >= 4.0 && freq <= 6.0) {
                out[i][0] = 0.0; 
                out[i][1] = 0.0;
            }
        }

        fftw_execute(p_backward);

        double raw_stft_output = in[SMC::WINDOW_SIZE - 1][0] / SMC::WINDOW_SIZE;
        double smoothed_output = (alpha * raw_stft_output) + ((1.0 - alpha) * previous_output);
        previous_output = smoothed_output;

        smoothed_output += fractional_remainder;
        int final_out = static_cast<int>(smoothed_output);
        fractional_remainder = smoothed_output - final_out;

        return final_out;
    }
};