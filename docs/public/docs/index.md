---
icon: lucide/rocket
---

# Overview

Welcome to the Proton Pass CLI documentation. The Proton Pass CLI is a command-line interface for managing your Proton Pass vaults, items, and secrets.

## Quick Start

<div style="background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%); border: 2px solid #0f3460; border-radius: 12px; padding: 32px; margin: 24px 0; box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);">
  <div style="text-align: center; margin-bottom: 28px;">
    <h3 style="margin: 0; font-family: 'Monaco', 'Menlo', 'Courier New', monospace; font-size: 24px; font-weight: bold; color: #6c63ff; text-shadow: 0 0 20px rgba(108, 99, 255, 0.5), 0 2px 4px rgba(0, 0, 0, 0.8); letter-spacing: 1px;">Get started in seconds</h3>
  </div>
  
  <div style="background: #0a0a0a; border: 1px solid #333; border-radius: 8px; padding: 24px; font-family: 'Monaco', 'Menlo', 'Courier New', monospace; font-size: 14px; line-height: 1.8; color: #e0e0e0; box-shadow: inset 0 2px 8px rgba(0, 0, 0, 0.6);">
    <div style="margin-bottom: 20px;">
      <div style="color: #888; margin-bottom: 6px; font-size: 12px; text-transform: uppercase; letter-spacing: 0.5px;">→ Download Pass CLI</div>
      <div class="command-line" style="display: flex; align-items: center; justify-content: space-between; position: relative;">
        <div style="color: #4ecca3; font-weight: 500; flex: 1;">curl -fsSL https://proton.me/download/pass-cli/install.sh | bash</div>
        <button onclick="copyToClipboard('curl -fsSL https://proton.me/download/pass-cli/install.sh | bash', this)" class="copy-btn" style="background: transparent; border: none; cursor: pointer; padding: 6px 10px; margin-left: 12px; border-radius: 6px; transition: all 0.2s ease; opacity: 0; display: flex; align-items: center; justify-content: center;" onmouseover="this.style.background='rgba(76, 204, 163, 0.15)'; this.style.opacity='1';" onmouseout="this.style.background='transparent'; this.style.opacity='0';">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#4ecca3" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
    </div>
    
    <div style="margin-bottom: 20px;">
      <div style="color: #888; margin-bottom: 6px; font-size: 12px; text-transform: uppercase; letter-spacing: 0.5px;">→ Log in</div>
      <div class="command-line" style="display: flex; align-items: center; justify-content: space-between; position: relative;">
        <div style="color: #4ecca3; font-weight: 500; flex: 1;">pass-cli login</div>
        <button onclick="copyToClipboard('pass-cli login', this)" class="copy-btn" style="background: transparent; border: none; cursor: pointer; padding: 6px 10px; margin-left: 12px; border-radius: 6px; transition: all 0.2s ease; opacity: 0; display: flex; align-items: center; justify-content: center;" onmouseover="this.style.background='rgba(76, 204, 163, 0.15)'; this.style.opacity='1';" onmouseout="this.style.background='transparent'; this.style.opacity='0';">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#4ecca3" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
    </div>
    
    <div>
      <div style="color: #888; margin-bottom: 6px; font-size: 12px; text-transform: uppercase; letter-spacing: 0.5px;">→ Start using it</div>
      <div class="command-line" style="display: flex; align-items: center; justify-content: space-between; position: relative;">
        <div style="color: #4ecca3; font-weight: 500; flex: 1;">pass-cli vault list</div>
        <button onclick="copyToClipboard('pass-cli vault list', this)" class="copy-btn" style="background: transparent; border: none; cursor: pointer; padding: 6px 10px; margin-left: 12px; border-radius: 6px; transition: all 0.2s ease; opacity: 0; display: flex; align-items: center; justify-content: center;" onmouseover="this.style.background='rgba(76, 204, 163, 0.15)'; this.style.opacity='1';" onmouseout="this.style.background='transparent'; this.style.opacity='0';">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#4ecca3" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
    </div>
  </div>
  
  <style>
    .command-line:hover .copy-btn {
      opacity: 1 !important;
    }
  </style>
  
  <script>
    function copyToClipboard(text, button) {
      navigator.clipboard.writeText(text).then(function() {
        const svg = button.querySelector('svg');
        const originalStroke = svg.getAttribute('stroke');
        
        // Change to checkmark
        svg.innerHTML = '<polyline points="20 6 9 17 4 12"></polyline>';
        svg.setAttribute('stroke', '#4ecca3');
        button.style.opacity = '1';
        
        // Reset after 2 seconds
        setTimeout(function() {
          svg.innerHTML = '<rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>';
          svg.setAttribute('stroke', originalStroke);
        }, 2000);
      }).catch(function(err) {
        console.error('Failed to copy text: ', err);
      });
    }
  </script>
</div>

- **[Installation](get-started/installation.md)** - Installation instructions for all platforms
- **[Getting Started](get-started/login.md)** - Login and configuration guides
- **[Pass Objects](objects/share.md)** - The different objects you can manage in Proton Pass
- **[Usage Guide](commands/login.md)** - Comprehensive guide to using the CLI

## What is Proton Pass CLI?

The Proton Pass CLI allows you to:

- **Manage vaults and items** - Create, list, view, and delete vaults and items from the command line
- **Inject secrets** - Use secrets in your applications via environment variables or template files
- **SSH integration** - Use Proton Pass-stored SSH keys with your existing SSH workflows
- **Automate workflows** - Integrate Proton Pass into your scripts and CI/CD pipelines

## Key Features

### Flexible Secret Management

- Reference secrets using a simple URI syntax: `pass://vault/item/field`
- Inject secrets into environment variables for your applications
- Process template files with secret references

### SSH Agent Integration

- Load SSH keys from Proton Pass into your existing SSH agent
- Run Proton Pass CLI as a standalone SSH agent
- Automatic key refresh and management

### Secure Key Storage

- Default keyring integration (macOS Keychain, Linux kernel keyring, Windows Credential Manager)
- Filesystem storage option for headless environments
- Encrypted session storage

## Need Help?

If you encounter any issues or have questions, please [contact us](https://proton.me/support/contact)
