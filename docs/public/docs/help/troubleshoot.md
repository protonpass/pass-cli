# Troubleshoot

## **I have an error creating client features. It cannot get the encryption key for the database**

This means you are running in an environment where there is no secure key ring we can reach to store encryption keys. We always try to save the local encryption keys in the safest location we can reach and that normally is the secure key ring that your operating system provides.

In this case you can use the local file system to store the local encryption keys. This is insecure and you should be aware that if somebody has access to your file system they can get to the encryption keys.

If you still want to use the app, you will need to:

 1. Ensure you are logged out by doing `pass-cli logout --force`
 2. Set the environment variable `PROTON_PASS_KEY_PROVIDER` to `fs`.
 3. Login normally as you would.

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

It will only allow running scripts that are signed, and the provided `install.ps1` is properly signed, so your computer should be able to run it without any further restrictions.

Once you have successfully installed it, you can set back the execution policy to its previous value by running the `Set-ExecutionPolicy` command again and passing the original value you got by running `Get-ExecutionPolicy`.

## Contact support

Head to our [support form](https://proton.me/support/contact) to get help from our fantastic support team.
