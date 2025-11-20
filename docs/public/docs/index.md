---
icon: lucide/rocket
---

# Overview

Welcome to the Proton Pass CLI documentation. The Proton Pass CLI is a command-line interface for managing your Proton Pass vaults, items, and secrets.

## Quick Start

<style>
  .cta-container {
    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
    border: 2px solid #0f3460;
    border-radius: 12px;
    padding: 32px;
    margin: 24px 0;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  }
  
  .cta-header {
    text-align: center;
    margin-bottom: 28px;
  }
  
  .cta-title {
    margin: 0;
    font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
    font-size: 24px;
    font-weight: bold;
    color: #6c63ff;
    text-shadow: 0 0 20px rgba(108, 99, 255, 0.5), 0 2px 4px rgba(0, 0, 0, 0.8);
    letter-spacing: 1px;
  }
  
  .terminal-box {
    background: #0a0a0a;
    border: 1px solid #333;
    border-radius: 8px;
    padding: 24px;
    font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
    font-size: 14px;
    line-height: 1.8;
    color: #e0e0e0;
    box-shadow: inset 0 2px 8px rgba(0, 0, 0, 0.6);
  }
  
  .command-section {
    margin-bottom: 20px;
  }
  
  .command-section:last-child {
    margin-bottom: 0;
  }
  
  .command-label {
    color: #888;
    margin-bottom: 6px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  
  .command-line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    position: relative;
  }
  
  .command-text {
    color: #4ecca3;
    font-weight: 500;
    flex: 1;
  }
  
  .copy-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 6px 10px;
    margin-left: 12px;
    border-radius: 6px;
    transition: all 0.2s ease;
    opacity: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  
  .copy-btn:hover {
    background: rgba(76, 204, 163, 0.15);
    opacity: 1;
  }
  
  .command-line:hover .copy-btn {
    opacity: 1;
  }
</style>

<div class="cta-container">
  <div class="cta-header">
    <h3 class="cta-title">Get started in seconds</h3>
  </div>
  
  <div class="terminal-box">
    <div class="command-section">
      <div class="command-label">→ Download Pass CLI</div>
      <div class="command-line">
        <div class="command-text">curl -fsSL https://proton.me/download/pass-cli/install.sh | bash</div>
        <button onclick="copyToClipboard('curl -fsSL https://proton.me/download/pass-cli/install.sh | bash', this)" class="copy-btn">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#4ecca3" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
    </div>
    
    <div class="command-section">
      <div class="command-label">→ Log in</div>
      <div class="command-line">
        <div class="command-text">pass-cli login</div>
        <button onclick="copyToClipboard('pass-cli login', this)" class="copy-btn">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#4ecca3" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
    </div>
    
    <div class="command-section">
      <div class="command-label">→ Start using it</div>
      <div class="command-line">
        <div class="command-text">pass-cli vault list</div>
        <button onclick="copyToClipboard('pass-cli vault list', this)" class="copy-btn">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#4ecca3" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
    </div>
  </div>
</div>

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

- **[Installation](get-started/installation.md)** - Installation instructions for all platforms
- **[Getting started](get-started/login.md)** - Login and configuration guides
- **[Pass objects](objects/share.md)** - The different objects you can manage in Proton Pass
- **[Usage guide](commands/login.md)** - Comprehensive guide to using the CLI

## What is Proton Pass CLI?

The Proton Pass CLI allows you to:

- **Manage vaults and items** - Create, list, view, and delete vaults and items from the command line
- **Inject secrets** - Use secrets in your applications via environment variables or template files
- **SSH integration** - Use Proton Pass-stored SSH keys with your existing SSH workflows
- **Automate workflows** - Integrate Proton Pass into your scripts and CI/CD pipelines

## Key features

### Flexible secret management

- Reference secrets using a simple URI syntax: `pass://vault/item/field`
- Inject secrets into environment variables for your applications
- Process template files with secret references

### SSH agent integration

- Load SSH keys from Proton Pass into your existing SSH agent
- Run Proton Pass CLI as a standalone SSH agent
- Automatic key refresh and management

### Secure key storage

- Default keyring integration (macOS Keychain, Linux kernel keyring, Windows Credential Manager)
- Filesystem storage option for headless environments
- Encrypted session storage

## Need help?

If you encounter any issues or have questions, please [contact us](https://proton.me/support/contact)
