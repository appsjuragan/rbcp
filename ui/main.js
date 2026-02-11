
document.addEventListener('DOMContentLoaded', () => {
    const { invoke } = window.__TAURI__.core;
    const { open } = window.__TAURI__.dialog;
    const { listen } = window.__TAURI__.event;

    // UI Elements
    const sourceInput = document.getElementById('source-path');
    const destInput = document.getElementById('dest-path');
    const browseSource = document.getElementById('browse-source');
    const browseFiles = document.getElementById('browse-files'); // NEW
    const browseDest = document.getElementById('browse-dest');
    const btnStart = document.getElementById('btn-start');
    const btnCancel = document.getElementById('btn-cancel');
    const btnPause = document.getElementById('btn-pause');
    const progressRing = document.getElementById('progress-ring');
    const progressPct = document.getElementById('progress-pct');
    const currentFileText = document.getElementById('current-file');
    const speedText = document.getElementById('speed');
    const fileCountText = document.getElementById('file-count');
    const statusText = document.getElementById('status-text');
    const logContent = document.getElementById('log-content');
    const clearLog = document.getElementById('clear-log');
    const toggleOptions = document.getElementById('toggle-options');
    const optionsPanel = document.getElementById('options-panel');
    const themeToggle = document.getElementById('theme-toggle');
    const threadSlider = document.getElementById('thread-count');
    const threadVal = document.getElementById('thread-val');
    const retrySlider = document.getElementById('retry-count');
    const retryVal = document.getElementById('retry-val');

    // Security: Disable common key combinations except essential ones
    document.addEventListener('keydown', (e) => {
        const isCtrl = e.ctrlKey || e.metaKey;
        const key = e.key.toLowerCase();

        // 1. Always allow standard text editing and selection
        const isCopy = isCtrl && key === 'c';
        const isPaste = isCtrl && key === 'v';
        const isCut = isCtrl && key === 'x';
        const isUndo = isCtrl && key === 'z' && !e.shiftKey;
        const isRedo = (isCtrl && key === 'y') || (isCtrl && key === 'z' && e.shiftKey);
        const isSelectAll = isCtrl && key === 'a';

        if (isCopy || isPaste || isCut || isUndo || isRedo || isSelectAll) {
            return;
        }

        // 2. Block system/browser shortcuts (Reload, Save, Print, Find, etc.)
        const prohibitedSystemKeys = ['r', 's', 'p', 'f', 'g', 'h', 'o', 'n', 't', 'w', 'j', 'u'];
        const isFunctionKey = e.key.startsWith('F');

        if ((isCtrl && prohibitedSystemKeys.includes(key)) || isFunctionKey) {
            e.preventDefault();
            return;
        }

        // 3. Prevent context menu key
        if (e.key === 'ContextMenu') {
            e.preventDefault();
        }
    });

    // Disable Right-Click context menu
    document.addEventListener('contextmenu', (e) => {
        // Allow context menu only for input fields if needed, 
        // but user requested to disable combinations/ux typical of browser
        e.preventDefault();
    }, false);

    // State
    let isRunning = false;
    let isPaused = false;
    let statusTimer = null;

    // Helpers
    const addLog = (msg) => {
        const div = document.createElement('div');
        const now = new Date().toLocaleTimeString([], { hour12: false });
        div.textContent = `[${now}] ${msg}`;
        logContent.appendChild(div);
        logContent.scrollTop = logContent.scrollHeight;
    };

    const setProgress = (pct) => {
        const radius = 45; // Fixed radius matching SVG
        const circumference = 2 * Math.PI * radius;
        const clampedPct = Math.min(100, Math.max(0, pct));
        const offset = circumference - (clampedPct / 100) * circumference;

        progressRing.style.strokeDasharray = `${circumference}`;
        progressRing.style.strokeDashoffset = `${offset}`;
        progressPct.textContent = `${Math.round(clampedPct)}%`;
    };

    const setStatus = (msg, color = 'var(--emerald)') => {
        statusText.textContent = msg;
        statusText.style.color = color;
        if (statusTimer) {
            clearTimeout(statusTimer);
            statusTimer = null;
        }
    };

    // Initialize progress bar
    setProgress(0);
    setStatus("ready");

    // Hide object count initially
    fileCountText.style.visibility = 'hidden';

    // Hide loader after initialization
    setTimeout(() => {
        const loader = document.getElementById('app-loader');
        if (loader) {
            loader.classList.add('hidden');
            setTimeout(() => loader.remove(), 300); // Remove after fade animation
        }
    }, 500);

    // Event Listeners
    browseSource.onclick = async () => {
        const defaultPath = localStorage.getItem('lastSourceDir');
        const selected = await open({
            directory: true,
            multiple: false,
            defaultPath: defaultPath || undefined
        });
        if (selected) {
            sourceInput.value = selected;
            localStorage.setItem('lastSourceDir', selected);
        }
    };

    browseFiles.onclick = async () => {
        const defaultPath = localStorage.getItem('lastSourceDir');
        const selected = await open({
            directory: false,
            multiple: true,
            defaultPath: defaultPath || undefined
        });
        if (selected) {
            if (Array.isArray(selected)) {
                if (selected.length > 0) {
                    // Selection successful
                }
                if (selected.length === 1) {
                    sourceInput.value = selected[0];
                } else {
                    sourceInput.value = selected.join(';');
                }
            } else {
                sourceInput.value = selected;
            }
        }
    };

    browseDest.onclick = async () => {
        const defaultPath = localStorage.getItem('lastDestDir');
        const selected = await open({
            directory: true,
            multiple: false,
            defaultPath: defaultPath || undefined
        });
        if (selected) {
            destInput.value = selected;
            localStorage.setItem('lastDestDir', selected);
        }
    };

    toggleOptions.onclick = () => {
        const isShown = optionsPanel.classList.toggle('show');
        const arrow = toggleOptions.querySelector('.arrow');
        if (arrow) {
            arrow.textContent = isShown ? '▲' : '▼';
        }
        // Force a layout recalculation or just confirm it worked
        if (isShown) {
            optionsPanel.style.display = 'block';
        } else {
            optionsPanel.style.display = 'none';
        }
    };

    threadSlider.oninput = () => {
        threadVal.textContent = threadSlider.value;
    };

    retrySlider.oninput = () => {
        retryVal.textContent = retrySlider.value;
    };

    themeToggle.onclick = () => {
        document.body.classList.toggle('dark-theme');
        document.body.classList.toggle('light-theme');
        themeToggle.textContent = document.body.classList.contains('dark-theme') ? '🌙' : '☀️';
    };

    clearLog.onclick = () => {
        logContent.innerHTML = '';
    };

    const showOverwriteModal = () => {
        const modal = document.getElementById('overwrite-modal');
        modal.classList.add('show');
        return new Promise((resolve) => {
            document.getElementById('modal-yes-all').onclick = () => {
                modal.classList.remove('show');
                resolve('overwrite');
            };
            document.getElementById('modal-no-all').onclick = () => {
                modal.classList.remove('show');
                resolve('skip');
            };
            document.getElementById('modal-cancel').onclick = () => {
                modal.classList.remove('show');
                resolve('cancel');
            };
        });
    };

    // Start Copy
    btnStart.onclick = async () => {
        const sourceVal = sourceInput.value;
        const dest = destInput.value;

        if (!sourceVal || !dest) {
            addLog("ERROR: Source and Destination must be specified.");
            return;
        }

        // Handle multiple sources separated by semicolon
        const sources = sourceVal.split(';').map(s => s.trim()).filter(s => s.length > 0);

        // Check for conflicts and ask user
        let overwriteMode = 'ask'; // 'ask', 'overwrite', 'skip'
        try {
            const hasConflicts = await invoke('check_conflicts', {
                sources: sources,
                destination: dest
            });

            if (hasConflicts) {
                const choice = await showOverwriteModal();

                if (choice === 'cancel') {
                    addLog("Operation cancelled by user.");
                    return;
                }
                overwriteMode = choice;
            }
        } catch (e) {
            // If check fails, proceed anyway
            addLog(`Note: Could not check for conflicts: ${e}`);
        }

        const options = {
            sources: sources,
            destination: dest,
            patterns: ["*.*"],
            recursive: document.getElementById('opt-recursive').checked,
            include_empty: document.getElementById('opt-recursive').checked,
            restartable: false,
            backup_mode: false,
            purge: document.getElementById('opt-mirror').checked,
            mirror: document.getElementById('opt-mirror').checked,
            move_files: document.getElementById('opt-move').checked,
            move_dirs: document.getElementById('opt-move').checked,
            attributes_add: "",
            attributes_remove: "",
            threads: parseInt(threadSlider.value),
            retries: parseInt(retrySlider.value),
            wait_time: 30,
            log_file: null,
            list_only: false,
            show_progress: true,
            log_file_names: true,
            empty_files: document.getElementById('opt-empty').checked,
            child_only: document.getElementById('opt-childonly').checked,
            shred_files: document.getElementById('opt-shred').checked,
            force_overwrite: overwriteMode === 'overwrite',
            preserve_root: true
        };

        try {
            isRunning = true;
            btnStart.disabled = true;
            btnCancel.disabled = false;
            btnPause.disabled = false;
            setStatus("waiting command...");
            btnStart.textContent = "Running...";
            fileCountText.style.visibility = 'visible'; // Show object count during copy

            await invoke('start_copy', { options });
            addLog("Initiating copy operation...");
            setStatus("scanning...");
        } catch (e) {
            addLog(`ERROR: ${e}`);
            isRunning = false;
            btnStart.disabled = false;
        }
    };

    btnCancel.onclick = async () => {
        await invoke('cancel_copy');
        addLog("Cancellation requested.");
    };

    btnPause.onclick = async () => {
        await invoke('toggle_pause');
        isPaused = !isPaused;
        btnPause.textContent = isPaused ? "Continue" : "Pause";
        addLog(isPaused ? "Operation paused." : "Operation resumed.");
    };

    // Tauri Events
    listen('copy-progress', (event) => {
        const info = event.payload;
        const pct = info.bytes_total === 0 ? 0 : (info.bytes_done / info.bytes_total) * 100;
        setProgress(pct);

        currentFileText.textContent = info.current_file || "Scanning...";
        speedText.textContent = `${(info.speed / 1024 / 1024).toFixed(2)} MB/s`;
        fileCountText.textContent = `${info.files_done} of ${info.files_total} objects`;

        if (info.state === 'Scanning') {
            setStatus("scanning...");
        } else if (info.state === 'Copying') {
            setStatus(isPaused ? "paused" : "copying...");
        } else if (info.state === 'Paused') {
            setStatus("paused", "var(--yellow)");
        }

        if (info.state === 'Completed' || info.state === 'Failed' || info.state === 'Cancelled') {
            isRunning = false;
            btnStart.disabled = false;
            btnCancel.disabled = true;
            btnPause.disabled = true;
            btnStart.textContent = "Start Copy";

            const finalStatus = info.state === 'Completed' ? "finished" : info.state.toLowerCase();
            const statusColor = info.state === 'Completed' ? 'var(--emerald)' : 'var(--red)';
            setStatus(finalStatus, statusColor);

            addLog(`Operation finished with state: ${info.state}`);

            // Reset back to "ready" after 10 seconds
            statusTimer = setTimeout(() => {
                setStatus("ready");
                currentFileText.textContent = "Ready to copy";
                fileCountText.style.visibility = 'hidden'; // Hide object count when idle
            }, 10000);
        }
    });

    listen('copy-log', (event) => {
        addLog(event.payload);
    });
});
