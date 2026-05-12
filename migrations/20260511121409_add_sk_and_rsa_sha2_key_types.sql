alter type ssh_key_type add value if not exists 'sk-ssh-ed25519@openssh.com';
alter type ssh_key_type add value if not exists 'sk-ecdsa-sha2-nistp256@openssh.com';
alter type ssh_key_type add value if not exists 'rsa-sha2-256';
alter type ssh_key_type add value if not exists 'rsa-sha2-512';
