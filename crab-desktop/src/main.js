const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

async function checkHealth() {
  try {
    const res = await invoke('check_health');
    document.getElementById('output').textContent = 'Health Check: ' + JSON.stringify(res, null, 2);
  } catch (e) {
    document.getElementById('output').textContent = 'Error: ' + e;
  }
}

async function sendTestMessage() {
  try {
    const input = document.getElementById('msg-input');
    const msg = input && input.value ? input.value : 'Hello from Tauri!';
    
    await invoke('send_test_message', { msg });
    document.getElementById('output').textContent = 'Message sent: ' + msg;
    
    // Clear input after send
    if (input) input.value = '';
  } catch (e) {
    document.getElementById('output').textContent = 'Error sending message: ' + e;
  }
}

async function getIpAddress() {
    try {
        const ip = await invoke('get_local_ip');
        const ipEl = document.getElementById('ip-address');
        if (ipEl) {
            ipEl.textContent = 'Local IP: ' + ip;
        }
    } catch (e) {
        console.error('Failed to get IP:', e);
        const ipEl = document.getElementById('ip-address');
        if (ipEl) {
            ipEl.textContent = 'Failed to get IP: ' + e;
        }
    }
}

async function exitApp() {
    // 商业交互逻辑：退出密码
    const password = prompt("🔒 Admin Access Required\nPlease enter the admin password to exit:", "");
    
    // 默认密码: 123456 (实际项目中应从配置读取)
    if (password === "123456") {
        if (confirm("⚠️ Warning: Are you sure you want to exit the POS system?")) {
            try {
                await invoke('exit_app');
            } catch (e) {
                console.error('Failed to exit:', e);
                alert('Failed to exit: ' + e);
            }
        }
    } else if (password !== null) {
        alert("❌ Access Denied: Incorrect password.");
    }
}

async function exportLogs() {
    try {
        const outputEl = document.getElementById('output');
        if (outputEl) outputEl.textContent = '📦 Packaging logs...';
        
        // Invoke Rust command to get zip bytes
        const logsZip = await invoke('export_logs');
        
        if (!logsZip || logsZip.length === 0) {
             alert('No logs found to export.');
             return;
        }

        // Create Blob from Uint8Array
        const blob = new Blob([new Uint8Array(logsZip)], { type: 'application/zip' });
        
        // Create download link
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        
        // Generate filename: crab_logs_YYYYMMDD_HHMMSS.zip
        const now = new Date();
        const timestamp = now.toISOString().replace(/[:.]/g, '-').slice(0, 19);
        a.download = `crab_logs_${timestamp}.zip`;
        
        document.body.appendChild(a);
        a.click();
        
        // Cleanup
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        
        if (outputEl) outputEl.textContent = '✅ Logs exported successfully!';
    } catch (e) {
        console.error('Export failed:', e);
        const outputEl = document.getElementById('output');
        if (outputEl) outputEl.textContent = '❌ Export failed: ' + e;
        alert('Export failed: ' + e);
    }
}

async function activateServer() {
    try {
        const auth_url = document.getElementById('act-auth-url').value;
        const tenant_id = document.getElementById('act-tenant').value;
        const common_name = document.getElementById('act-common-name').value;
        const username = document.getElementById('act-user').value;
        const password = document.getElementById('act-pass').value;

        if (!auth_url || !tenant_id || !common_name || !username || !password) {
            alert("Please fill all activation fields");
            return;
        }

        const res = await invoke('activate_server', { 
            params: {
                username,
                password,
                auth_url,
                tenant_id,
                common_name,
                role: "server"
            }
        });
        document.getElementById('output').textContent = 'Activation: ' + res;
    } catch (e) {
        document.getElementById('output').textContent = 'Error activating: ' + e;
    }
}

window.addEventListener("DOMContentLoaded", () => {
  const btnHealth = document.getElementById('btn-health');
  const btnSend = document.getElementById('btn-send');
  const btnExit = document.getElementById('btn-exit');
  const btnExport = document.getElementById('btn-logs');
  const btnActivate = document.getElementById('btn-activate');

  if (btnHealth) {
    btnHealth.addEventListener('click', checkHealth);
  }
  if (btnSend) {
    btnSend.addEventListener('click', sendTestMessage);
  }
  if (btnExit) {
    btnExit.addEventListener('click', exitApp);
  }
  if (btnExport) {
    btnExport.addEventListener('click', exportLogs);
  }
  if (btnActivate) {
    btnActivate.addEventListener('click', activateServer);
  }

  getIpAddress();

  // Retry getting IP every 2 seconds if failed (common on mobile start)
  let retryCount = 0;
  const ipInterval = setInterval(() => {
    const ipEl = document.getElementById('ip-address');
    if (ipEl && (ipEl.textContent.includes('Loading') || ipEl.textContent.includes('Failed')) && retryCount < 5) {
        getIpAddress();
        retryCount++;
    } else {
        clearInterval(ipInterval);
    }
  }, 2000);

  // Listen for server broadcasts
  listen('server-message', (event) => {
    console.log('Received server message:', event.payload);
    
    // Log raw output
    const outputEl = document.getElementById('output');
    if (outputEl) {
        const current = outputEl.textContent;
        // Keep only last few logs to avoid overflow
        const logs = current.split('\n\n').slice(-5);
        logs.push('Received: ' + JSON.stringify(event.payload));
        outputEl.textContent = logs.join('\n\n');
        outputEl.scrollTop = outputEl.scrollHeight;
    }

    const msg = event.payload;
    // Check if it's a notification
    // event_type might be "notification" or "Notification" depending on serialization
    if (msg.event_type && msg.event_type.toLowerCase() === 'notification') {
        let data = msg.payload || {};
        
        // Handle payload parsing if it's a raw array (from BusMessage)
        if (Array.isArray(data)) {
            try {
                // Convert byte array to string then JSON
                const text = new TextDecoder().decode(new Uint8Array(data));
                data = JSON.parse(text);
            } catch (e) {
                // If not JSON, use raw text
                data = { body: new TextDecoder().decode(new Uint8Array(data)) };
            }
        }
        
        const title = data.title || 'Notification';
        const body = data.body || JSON.stringify(data);
        const time = data.timestamp ? new Date(data.timestamp).toLocaleTimeString() : new Date().toLocaleTimeString();
        
        const container = document.getElementById('notifications-container');
        if (container) {
            const card = document.createElement('div');
            card.className = 'notification-card';
            card.innerHTML = `
                <div class="notification-header">
                    <span>${title}</span>
                    <span class="notification-time">${time}</span>
                </div>
                <div class="notification-body">${body}</div>
            `;
            // Append to show chronological order (Append mode)
            container.appendChild(card);
            // Scroll into view
            card.scrollIntoView({ behavior: 'smooth', block: 'end' });
        }
    }
    
    // Show system notification if supported
    if (window.Notification && Notification.permission === "granted") {
       const title = msg.payload?.title || "New Message";
       const body = msg.payload?.body || JSON.stringify(msg.payload);
       new Notification(title, { body });
    } else if (window.Notification && Notification.permission !== "denied") {
        Notification.requestPermission().then(permission => {
            if (permission === "granted") {
                const title = msg.payload?.title || "New Message";
                const body = msg.payload?.body || JSON.stringify(msg.payload);
                new Notification(title, { body });
            }
        });
    }
  });
});
