---
icon: lucide/rocket
hide:
  - toc
---

!!! warning "Proton Pass CLI is in Beta"
    Currently, the Proton Pass CLI is in beta status, and is only available for some users during this beta period. We plan to make it available to more users soon, so stay tuned for more news!

# Overview

Welcome to the Proton Pass CLI documentation. The Proton Pass CLI is a command-line interface for managing your Proton Pass vaults, items, and secrets.

<style>
  /* Hide left sidebar and make content full width */
  .md-sidebar--primary {
    display: none !important;
  }

  .md-content__inner {
    max-width: 100% !important;
  }

  /* Hide mobile hamburger menu icon */
  .md-header__button.md-logo {
    display: none !important;
  }

  .md-header__button[for="__drawer"] {
    display: none !important;
  }

  .cta-container {
    background: #f8f9fa;
    border: 1px solid #e1e4e8;
    border-radius: 6px;
    padding: 24px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .cta-header {
    text-align: center;
    margin-bottom: 20px;
  }

  .cta-title {
    margin: 0 !important;
    font-size: 20px;
    font-weight: 600;
    color: #24292f;
    letter-spacing: -0.01em;
  }

  .terminal-box {
    background: #1e1e2e;
    border: 1px solid #2d2d44;
    border-radius: 6px;
    padding: 20px;
    font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
    font-size: 14px;
    line-height: 1.8;
    color: #cdd6f4;
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.3), 0 2px 8px rgba(0, 0, 0, 0.2);
  }

  .command-section {
    margin-bottom: 16px;
  }

  .command-section:last-child {
    margin-bottom: 0;
  }

  .command-label {
    color: #a6adc8;
    margin-bottom: 8px;
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0;
  }

  .command-line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    position: relative;
    cursor: pointer;
    border-radius: 4px;
    padding: 4px 0;
    transition: background-color 0.15s ease;
  }

  .command-line:hover {
    background: rgba(148, 226, 213, 0.08);
  }

  .command-text {
    color: #94e2d5;
    font-weight: 400;
    flex: 1;
    user-select: none;
  }

  .copy-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 6px 10px;
    margin-left: 12px;
    border-radius: 4px;
    transition: all 0.15s ease;
    opacity: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }

  .copy-btn:hover {
    background: rgba(148, 226, 213, 0.15);
  }

  .command-line:hover .copy-btn {
    opacity: 1;
    pointer-events: auto;
  }

  .copy-btn svg {
    stroke: #a6adc8;
  }

  [data-md-color-scheme="slate"] .cta-container {
    background: #1a1a1a;
    border: 1px solid #2d2d2d;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }

  [data-md-color-scheme="slate"] .cta-title {
    color: #e4e4e7;
  }

  .tabs-container {
    display: flex;
    gap: 8px;
    margin-bottom: 20px;
    border-bottom: 1px solid #e1e4e8;
    padding-bottom: 8px;
  }

  [data-md-color-scheme="slate"] .tabs-container {
    border-bottom: 1px solid #2d2d2d;
  }

  .tab-button {
    background: transparent;
    border: none;
    padding: 8px 16px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    color: #6e7781;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .tab-button:hover {
    background: rgba(148, 226, 213, 0.08);
    color: #24292f;
  }

  [data-md-color-scheme="slate"] .tab-button:hover {
    color: #e4e4e7;
  }

  .tab-button.active {
    background: rgba(148, 226, 213, 0.15);
    color: #24292f;
    font-weight: 600;
  }

  [data-md-color-scheme="slate"] .tab-button.active {
    color: #e4e4e7;
  }

  .tooltip {
    position: absolute;
    background: #1e1e2e;
    color: #cdd6f4;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transform: translateX(-50%) translateY(-100%);
    top: -8px;
    left: 50%;
    transition: opacity 0.2s ease;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.4);
    border: 1px solid #313244;
    z-index: 10;
  }

  .command-line:hover .tooltip {
    opacity: 0.95;
  }

  .tooltip::after {
    content: '';
    position: absolute;
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
    border: 6px solid transparent;
    border-top-color: #1e1e2e;
  }
</style>

<div class="cta-container">
  <div class="cta-header">
    <h3 class="cta-title">Get started in seconds</h3>
  </div>

  <div class="tabs-container">
    <button class="tab-button active" onclick="switchTab('unix')">Linux/macOS</button>
    <button class="tab-button" onclick="switchTab('windows')">Windows</button>
  </div>

  <div class="terminal-box">
    <div class="command-section" id="download-command">
      <div class="command-label">→ Download Pass CLI</div>
      <div class="command-line unix-command" onclick="copyToClipboard('curl -fsSL https://proton.me/download/pass-cli/install.sh | bash', this)">
        <span class="tooltip">Click to copy</span>
        <div class="command-text">curl -fsSL https://proton.me/download/pass-cli/install.sh | bash</div>
        <button class="copy-btn">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
      <div class="command-line windows-command" style="display: none;" onclick="copyToClipboard('Invoke-WebRequest -Uri https://proton.me/download/pass-cli/install.ps1 -OutFile install.ps1; .\\install.ps1', this)">
        <span class="tooltip">Click to copy</span>
        <div class="command-text">Invoke-WebRequest -Uri https://proton.me/download/pass-cli/install.ps1 -OutFile install.ps1; .\install.ps1</div>
        <button class="copy-btn">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
    </div>

    <div class="command-section">
      <div class="command-label">→ Log in</div>
      <div class="command-line" onclick="copyToClipboard('pass-cli login', this)">
        <span class="tooltip">Click to copy</span>
        <div class="command-text">pass-cli login</div>
        <button class="copy-btn">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
    </div>

    <div class="command-section">
      <div class="command-label">→ Start using it</div>
      <div class="command-line" onclick="copyToClipboard('pass-cli vault list', this)">
        <span class="tooltip">Click to copy</span>
        <div class="command-text">pass-cli vault list</div>
        <button class="copy-btn">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
      </div>
    </div>
  </div>
</div>

<script>
  function copyToClipboard(text, commandLine) {
    navigator.clipboard.writeText(text).then(function() {
      const copyBtn = commandLine.querySelector('.copy-btn');
      const svg = copyBtn.querySelector('svg');
      const originalStroke = svg.getAttribute('stroke');
      const checkmarkColor = '#94e2d5';

      // Change to checkmark
      svg.innerHTML = '<polyline points="20 6 9 17 4 12"></polyline>';
      svg.setAttribute('stroke', checkmarkColor);
      copyBtn.style.opacity = '1';

      // Reset after 2 seconds
      setTimeout(function() {
        svg.innerHTML = '<rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>';
        svg.setAttribute('stroke', originalStroke);
      }, 2000);
    }).catch(function(err) {
      console.error('Failed to copy text: ', err);
    });
  }

  function switchTab(platform) {
    // Update tab button states
    document.querySelectorAll('.tab-button').forEach(function(btn) {
      btn.classList.remove('active');
    });
    event.target.classList.add('active');

    // Show/hide appropriate commands
    const unixCommands = document.querySelectorAll('.unix-command');
    const windowsCommands = document.querySelectorAll('.windows-command');

    if (platform === 'unix') {
      unixCommands.forEach(function(cmd) { cmd.style.display = 'flex'; });
      windowsCommands.forEach(function(cmd) { cmd.style.display = 'none'; });
    } else if (platform === 'windows') {
      unixCommands.forEach(function(cmd) { cmd.style.display = 'none'; });
      windowsCommands.forEach(function(cmd) { cmd.style.display = 'flex'; });
    }
  }

  // Prevent button clicks from triggering the command-line click
  document.addEventListener('DOMContentLoaded', function() {
    document.querySelectorAll('.copy-btn').forEach(function(btn) {
      btn.addEventListener('click', function(e) {
        e.stopPropagation();
        const commandLine = btn.closest('.command-line');
        const commandText = commandLine.querySelector('.command-text').textContent;
        copyToClipboard(commandText, commandLine);
      });
    });
  });
</script>

Did you have any issue during the installation? Take a look at our [Troubleshooting page](./help/troubleshoot.md).

## Quick start

- **[Installation](get-started/installation.md)** - Installation instructions for all platforms
- **[Pass objects](objects/share.md)** - The different objects you can manage in Proton Pass
- **[Reference](commands/login.md)** - Reference guide to the commands the Pass CLI has to offer

## What is Proton Pass CLI?

The Proton Pass CLI allows you to manage your Proton Pass vaults and items directly from the command line, bringing the power of secure secret management to your terminal workflow. With the CLI, you can create, list, view, and delete vaults and items seamlessly, making it an great tool for developers and system administrators who prefer working in the command line.

Beyond basic vault management, the CLI allows you to inject secrets into your applications through environment variables or template files, enabling easy integration with your deployment workflows. The tool also provides comprehensive SSH integration, allowing you to use your SSH keys stored in Proton Pass with your existing SSH workflows.

## Key features

### Flexible secret management

The CLI offers a flexible and intuitive approach to secret management through a simple URI syntax. You can reference any secret using the format `pass://vault/item/field`, making it easy to access specific credentials programmatically. This design allows you to inject secrets into environment variables for your applications or process template files that contain secret references.

### SSH agent integration

For developers working with SSH keys, the CLI provides robust SSH agent integration capabilities. You can load SSH keys stored in Proton Pass directly into your existing SSH agent, or run the Proton Pass CLI as a standalone SSH agent.

## Need help?

If you encounter any issues or have questions, please [contact us](https://proton.me/support/contact)
