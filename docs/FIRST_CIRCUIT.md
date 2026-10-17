# Your first circuit in CircuitCraft

Use the bundled RC low-pass example to explore the schematic, simulation and PCB views. This walkthrough is for the Windows **0.3.0** release.

## 1. Open the complete Windows package

[Download CircuitCraft 0.3.0 for Windows x64](https://github.com/Berserk-hub150/CircuitCraft/releases/download/v0.3.0/CircuitCraft-0.3.0-Windows-x64.zip), extract the entire ZIP and run `Start-CircuitCraft.bat`.

Keep the subfolders together. They contain the ngspice engine and supporting files. Windows 10/11 x64 and a working graphics driver are required.

## 2. Run the RC example

The RC low-pass schematic opens at startup. The same project is available as [examples/rc-lowpass.cforge](../examples/rc-lowpass.cforge).

Press **F5** to run the AC frequency response. Inspect the **VOUT** trace in the waveform view.

The example uses **R1 = 1 kΩ** and **C1 = 1 µF**. For this ideal filter, the cutoff frequency is:

`fc = 1 / (2 × pi × R × C) ≈ 159.15 Hz`

The output magnitude should fall as frequency increases. For the example's unit AC input, the output magnitude is about **−3.01 dB** at the cutoff. Use a dB representation when checking that value.

## 3. Change one component

Select **R1**, change its value from `1k` to `2k` and run the same AC analysis again. The theoretical cutoff halves to about **79.58 Hz**.

Use **Ctrl+Z** to undo the edit, or reopen the original example. Keep the first project unchanged by saving experiments under another filename.

For a direct overlay, use **Waveforms → Component parameter sweep**, select **R1** and enter `500,1k,2k`. Run the sweep to compare all three responses. See [Advanced features](ADVANCED_FEATURES.md) for the controls and measurement definitions.

## 4. Inspect the PCB

Open **PCB layout** to inspect the example board. Its example tracks are already present. If you change schematic connectivity, use **Synchronize from schematic** and inspect the resulting connections and design-rule messages.

To explore the autorouter, use a copy of your project with unrouted connections and choose **Route unrouted connections**. It is bounded and can leave connections unresolved; inspect the remaining ratsnest and DRC results.

Open **PCB 3D**:
- Drag to orbit.
- Use the mouse wheel to zoom.
- Middle-drag to pan.
- Use **Top / Bottom / Isometric** to change the view.

The 3D bodies are approximate component envelopes. See [Engineering workflows](ENGINEERING_WORKFLOWS.md) for the routing, hierarchy and 3D limits.

## 5. Share a reproducible result

[Open an issue](https://github.com/Berserk-hub150/CircuitCraft/issues/new/choose) with:
- CircuitCraft version and operating system.
- The steps you followed.
- What you expected and what happened.
- A small `.cforge` file, screenshot or error message where relevant.

A report such as “R1 = 2k, C1 = 1u, AC analysis: expected cutoff near 79.58 Hz” is useful for checking the numerical result.

[Back to README](../README.md)
