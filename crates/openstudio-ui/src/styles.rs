pub const FL_STUDIO_CSS: &str = r#"
/* Aurora Studio - Tema Escuro Fiel ao FL Studio 21/24 */

window.main-window {
    background-color: #17191d;
    color: #c8d1dc;
    font-family: "Segoe UI", "Inter", "Ubuntu", sans-serif;
}

/* Barra de Transporte Superior Compacta estilo FL Studio */
.transport-bar {
    background-color: #23272e;
    border-bottom: 2px solid #101215;
    padding: 4px 10px;
    min-height: 40px;
}

/* Displays Digitais LCD de Alta Resolução */
.lcd-display {
    background-color: #0f1114;
    border: 1px solid #2b313a;
    border-radius: 3px;
    padding: 3px 8px;
    font-family: "Monospace", "Courier New", monospace;
    font-weight: 800;
}

.lcd-bpm {
    color: #ff9f43;
    font-size: 14px;
    text-shadow: 0 0 6px rgba(255, 159, 67, 0.5);
}

.lcd-time {
    color: #00d2d3;
    font-size: 14px;
    text-shadow: 0 0 6px rgba(0, 210, 211, 0.5);
}

.lcd-meter {
    color: #2ed573;
    font-size: 11px;
}

/* Botões de Transporte */
button.transport-btn {
    background-color: #2c323b;
    color: #d1d8e0;
    border: 1px solid #3c4450;
    border-radius: 3px;
    padding: 4px 10px;
    font-size: 11px;
    font-weight: 700;
}

button.transport-btn:hover {
    background-color: #38404c;
    border-color: #4b5563;
}

button.transport-btn.play-btn:checked,
button.transport-btn.play-btn.active {
    background-color: #ff851b;
    color: #121417;
    border-color: #ffa502;
    box-shadow: 0 0 10px rgba(255, 133, 27, 0.7);
}

button.transport-btn.record-btn:checked,
button.transport-btn.record-btn.active {
    background-color: #ff4757;
    color: #ffffff;
    border-color: #ff6b81;
    box-shadow: 0 0 10px rgba(255, 71, 87, 0.7);
}

/* Alternador SONG / PAT Estilo FL Studio */
.mode-switch {
    margin-left: 4px;
    margin-right: 6px;
}

.mode-switch button {
    background-color: #1a1e24;
    color: #8395a7;
    border: 1px solid #2d343e;
    font-size: 11px;
    font-weight: 900;
    padding: 4px 10px;
    min-width: 44px;
}

.mode-switch button:first-child {
    border-top-left-radius: 4px;
    border-bottom-left-radius: 4px;
}

.mode-switch button:last-child {
    border-top-right-radius: 4px;
    border-bottom-right-radius: 4px;
    border-left: none;
}

.mode-switch button.active {
    background-color: #ff851b;
    color: #111315;
    border-color: #ffa502;
    box-shadow: 0 0 10px rgba(255, 133, 27, 0.7);
}

/* Badge de Hardware / GPU */
.gpu-badge {
    background-color: #1c262b;
    border: 1px solid #10ac84;
    border-radius: 10px;
    padding: 2px 10px;
    font-size: 11px;
    color: #2ed573;
    font-weight: 700;
}

/* Botões da Barra de Janelas (F5 a F9) */
button.window-toggle-btn {
    background-color: #282e36;
    color: #cbd5e1;
    border: 1px solid #3b434e;
    border-radius: 3px;
    padding: 4px 10px;
    font-size: 11px;
    font-weight: 700;
}

button.window-toggle-btn:hover {
    background-color: #343b46;
}

button.window-toggle-btn:checked,
button.window-toggle-btn.active {
    background-color: #54a0ff;
    color: #0f172a;
    border-color: #70a1ff;
    box-shadow: 0 0 8px rgba(84, 160, 255, 0.6);
}

/* Área de Trabalho Central da DAW */
.workspace-box {
    background-color: #14171a;
    padding: 6px;
}

/* Painel do Channel Rack */
.channel-rack-box {
    background-color: #20242a;
    border: 1px solid #2f3640;
    border-radius: 4px;
    padding: 6px;
}

.channel-rack-header {
    background-color: #1c2026;
    border-bottom: 1px solid #2b313a;
    padding: 4px 8px;
    margin-bottom: 4px;
}

.channel-strip {
    background-color: #272c33;
    border-bottom: 1px solid #1a1e23;
    border-radius: 3px;
    padding: 3px 6px;
    margin-bottom: 2px;
}

.channel-name-btn {
    background-color: #353c46;
    color: #f1f2f6;
    border: 1px solid #454f5d;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 700;
    min-width: 95px;
    padding: 4px 8px;
}

.channel-name-btn:hover {
    background-color: #424c58;
}

.channel-trk-badge {
    background-color: #171a1f;
    border: 1px solid #2f3640;
    border-radius: 2px;
    color: #ff9f43;
    font-family: monospace;
    font-size: 11px;
    font-weight: 800;
    padding: 2px 6px;
}

/* Botões 3D do Step Sequencer com chanfros característicos */
.step-button {
    min-width: 28px;
    min-height: 30px;
    border-radius: 3px;
    padding: 0;
    margin: 1px;
}

/* Batidas 1 e 3 (Cinza Escuro com relevo) */
.step-dark {
    background-color: #323842;
    border-top: 1px solid #454d5a;
    border-left: 1px solid #454d5a;
    border-right: 1px solid #1f232a;
    border-bottom: 1px solid #1f232a;
}

.step-dark:hover {
    background-color: #3e4652;
}

/* Batidas 2 e 4 (Cinza Médio com relevo) */
.step-light {
    background-color: #474f5d;
    border-top: 1px solid #5a6475;
    border-left: 1px solid #5a6475;
    border-right: 1px solid #2a2f38;
    border-bottom: 1px solid #2a2f38;
}

.step-light:hover {
    background-color: #555e6e;
}

/* Step Ativo (Neon Amber do FL Studio) */
.step-active {
    background-color: #ff9f43;
    border-top: 1px solid #ffbe76;
    border-left: 1px solid #ffbe76;
    border-right: 1px solid #e58e26;
    border-bottom: 1px solid #e58e26;
    box-shadow: 0 0 10px rgba(255, 159, 67, 0.9);
}

/* Playhead ativo passando pelo step */
.step-playhead {
    border-top: 2px solid #2ed573;
    box-shadow: 0 -2px 6px rgba(46, 213, 115, 0.8);
}

/* Piano Roll Container */
.piano-roll-container {
    background-color: #1a1e24;
    border: 1px solid #2c333d;
    border-radius: 4px;
    padding: 4px;
}

.piano-roll-toolbar {
    background-color: #1f232a;
    border-bottom: 1px solid #2d3440;
    padding: 4px 8px;
    border-radius: 3px;
    margin-bottom: 4px;
}

.piano-roll-dropdown {
    background-color: #2b313b;
    color: #e2e8f0;
    border: 1px solid #3d4654;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 700;
    padding: 2px 8px;
}

/* Mixer Container */
.mixer-container {
    background-color: #1e2228;
    border: 1px solid #2d333c;
    border-radius: 4px;
    padding: 6px;
}

.mixer-track-box {
    background-color: #262b32;
    border: 1px solid #363c46;
    border-radius: 3px;
    padding: 4px;
    min-width: 68px;
}

.mixer-track-master {
    background-color: #1e2735;
    border-color: #54a0ff;
}

.mixer-track-name {
    font-size: 11px;
    font-weight: 700;
    color: #e2e8f0;
}

.mixer-slot-row {
    background-color: #181c22;
    border: 1px solid #282f3a;
    border-radius: 3px;
    padding: 3px 6px;
    font-size: 11px;
    color: #94a3b8;
}

/* Navegador Lateral / Browser */
.browser-tree {
    background-color: #181b20;
    border-right: 1px solid #272c35;
    color: #cbd5e1;
    font-size: 12px;
}
"#;

