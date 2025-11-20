# Troubleshoot

## **I have an error creating client features. It cannot get the encryption key for the database**

This means you are running in an environment where there is no secure key ring we can reach to store encryption keys. We always try to save the local encryption keys in the safest location we can reach and that normally is the secure key ring that your operating system provides.

In this case you can use the local file system to store the local encryption keys. This is insecure and you should be aware that if somebody has access to your file system they can get to the encryption keys.

If you still want to use the app, you will need to:

 1. Ensure you are logged out by doing `pass-cli logout --force`
 2. Set the environment variable `PROTON_PASS_KEY_PROVIDER` to `fs`.
 3. Login normally as you would.

## Contact support

 Head to our [support form](https://proton.me/support/contact) to get help from our fantastic support team.
