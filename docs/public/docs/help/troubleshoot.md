# Troubleshoot

## **I have an error creating client features. It cannot get the encryption key for the database**

This means you are running in an environment where there is no secure key ring we can reach to store encryption keys. We always try to save the local encryption keys in the safest location we can reach and that normally is the secure key ring that your operating system provides.

In this case you can use the local file system to store the local encryption keys. This is insecure and you should be aware that if somebody has access to your file system they can get to the encryption keys.

If you still want to use the app, you will need to:

 1. Ensure you are logged out by doing `pass-cli logout --force`
 2. Set the environment variable `PROTON_PASS_KEY_PROVIDER` to `fs`.
 3. Login normally as you would.

## **Linux: keyring error with `NoStorageAccess` or D-Bus errors**

On Linux, the CLI uses the kernel keyring by default, which does not require D-Bus. However, if you have explicitly set `PROTON_PASS_LINUX_KEYRING=dbus` and see an error like:

```
Error accessing credential [...]: NoStorageAccess(Unknown(1))
```

this means the D-Bus Secret Service (e.g. GNOME Keyring) is unavailable or has not been unlocked yet. Common causes:

- You are in a desktop session but have not yet unlocked the GNOME Keyring (e.g. first login, or the keyring was manually locked).
- You are connecting over SSH without forwarding a D-Bus session socket.
- The Secret Service daemon is not running.

**Solutions:**

- **Unlock your desktop session**: log in to your graphical session to unlock GNOME Keyring, then retry.
- **Switch back to the kernel keyring** (default): unset or remove `PROTON_PASS_LINUX_KEYRING` from your environment:
  ```bash
  unset PROTON_PASS_LINUX_KEYRING
  ```
- **Use filesystem key storage** for headless/SSH environments where neither backend is accessible:
  ```bash
  pass-cli logout --force
  export PROTON_PASS_KEY_PROVIDER=fs
  pass-cli login
  ```

See the [Configuration - Linux keyring note](../get-started/configuration.md#linux-keyring-note) for a full explanation of the available backends.

## **On Windows it complains about `install.ps1` cannot be loaded because running scripts is disabled**

It's possible that your computer has a restricted script execution policy set, either by you or via a company Device Management System.

In order to check if that's the case for the execution of the script, you should open `Powershell` in Administrator mode, and run:

```powershell
Get-ExecutionPolicy
```

There you will be able to check the current execution policy. In order to allow the installation of the script, you can run this command:

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope LocalMachine
```

It will only allow to run scripts that are signed, and the provided `install.ps1` is properly signed, so your computer should be able to run it without any further restrictions.

Once you have successfully installed it, you can set back the execution policy to its previous value by running back again the `Set-ExecutionPolicy` command and passing the original value you got by running `Get-ExecutionPolicy`. 

## SSH Agent troubleshooting

If you have issues with your SSH Agent in Proton Pass CLI, please try the following steps to see if a minimal scenario works on your setup:

1. If you are using Windows, make sure that the system default OpenSSH agent is disabled. To do so, press `Windows+R`, write `services.msc`, click OK, and then look for "OpenSSH Authentication Agent". Right-click it and select "Properties". There, set "Startup type" to "Disabled" and click OK. Make sure the service is now marked as Stopped.
2. Select a vault / Create a new temporary one to hold a new temporary SSH key.
3. Create a new SSH key in that vault: `pass-cli item create ssh-key generate --title "TempSshKey"`.
4. Start the SSH agent with debug logs: (macOS/Linux) `PASS_LOG_LEVEL=debug pass-cli ssh-agent start --vault-name YOUR_VAULT_NAME` / (Windows) `$env:PASS_LOG_LEVEL="debug"; pass-cli ssh-agent start --vault-name YOUR_VAULT_NAME`.
5. Open a new shell. If you are using macOS or Linux, run the `export` command that got printed on step 4. Then run `ssh-add -L`. If everything is working correctly, you should see your `TempSshKey` listed there, and a log message appear in your SSH Agent shell.

## Contact support

Head to our [support form](https://proton.me/support/contact) to get help from our fantastic support team.
