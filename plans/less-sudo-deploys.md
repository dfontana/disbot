# Reducing Sudo Password Prompts in Deployment

## Problem Description

The current deployment script (`bin/deploy`) requires sudo password input multiple times during each deployment:
- Line 65: `sudo systemctl stop disbot-$env`
- Line 67: `sudo systemctl restart disbot-$env`

This interrupts the automated deployment process and requires manual intervention.

## Current Deploy Script Analysis

The deploy script performs these sudo operations:
1. Stops the systemd service
2. Copies the new binary 
3. Restarts the systemd service

Each SSH session with sudo prompts for password independently.

## Solution Options

### Option 1: Passwordless Sudo (Recommended)

**Implementation:**
```bash
# Create sudoers file on remote server
sudo visudo -f /etc/sudoers.d/disbot-deploy

# Add content:
deploy_user ALL=(ALL) NOPASSWD: /bin/systemctl stop disbot-*, /bin/systemctl restart disbot-*, /bin/systemctl start disbot-*
```

**Security Analysis:**
- ✅ **Pros:** Highly restrictive, only specific commands, audit trail maintained
- ❌ **Risks:** If deploy user compromised, attacker can control disbot services
- 🔒 **Mitigation:** Use dedicated deploy user, SSH keys only, monitor logs

**More Restrictive Alternative:**
```bash
# Even more specific - only allow exact service names
deploy_user ALL=(ALL) NOPASSWD: /bin/systemctl stop disbot-prod, /bin/systemctl restart disbot-prod, /bin/systemctl stop disbot-dev, /bin/systemctl restart disbot-dev
```

### Option 2: SSH Connection Multiplexing

**Implementation:**
```bash
# Add to ~/.ssh/config or deploy script
Host deployment-server
    ControlMaster auto
    ControlPath ~/.ssh/sockets/%r@%h-%p
    ControlPersist 600
```

**Deploy Script Changes:**
```bash
# Establish master connection
ssh -fN $USER@$host

# All subsequent SSH commands reuse connection
# Still need passwordless sudo or will prompt once
```

**Benefits:**
- Faster subsequent connections
- Reduces connection overhead
- Can combine with passwordless sudo

### Option 3: Single SSH Session Approach

**Implementation:**
```bash
# Combine all remote operations into one SSH call
ssh -t $USER@$host "
    sudo systemctl stop disbot-$env &&
    # Wait for file copy completion signal
    while [ ! -f ~/deploy/disbot-$env.ready ]; do sleep 0.1; done &&
    sudo systemctl restart disbot-$env
"
```

**Challenges:**
- Complex coordination between file copy and service restart
- Error handling becomes more difficult
- Still requires passwordless sudo or prompts once

### Option 4: Service User Approach (Alternative Architecture)

**Implementation:**
- Run disbot services as the deploy user (not root)
- Use systemd user services instead of system services
- No sudo required at all

**Changes Required:**
```bash
# User service files in ~/.config/systemd/user/
systemctl --user stop disbot-prod
systemctl --user restart disbot-prod
```

**Benefits:**
- No sudo required
- Better security isolation
- Deploy user owns the process

**Considerations:**
- Services won't start on boot unless `loginctl enable-linger deploy_user`
- May need to adjust service configurations
- Network port binding restrictions (<1024 requires privileges)

### Option 5: Process Manager Approach

**Implementation:**
- Use PM2, supervisor, or similar process manager
- Deploy user manages processes directly
- No systemd/sudo required

**Example with PM2:**
```bash
pm2 stop disbot-prod
pm2 restart disbot-prod
```

## Recommended Implementation Plan

### Phase 1: Immediate Solution (Passwordless Sudo)
1. Create dedicated deploy user on target server
2. Configure SSH key authentication for deploy user
3. Create restrictive sudoers entry for specific systemctl commands
4. Test deployment process

### Phase 2: Enhanced Security (Connection Multiplexing)
1. Configure SSH connection multiplexing
2. Optimize deploy script for connection reuse
3. Add proper error handling

### Phase 3: Long-term (Service User Architecture)
1. Evaluate converting to user services
2. Test service startup/management as non-root user  
3. Update deployment scripts accordingly

## Security Best Practices

1. **Dedicated Deploy User**
   - Create user solely for deployment
   - Minimal shell access
   - SSH key authentication only

2. **Restrictive Sudoers**
   - Specific command paths only
   - No wildcards where possible
   - Regular sudo log monitoring

3. **SSH Security**
   - Key-based authentication
   - Disable password auth for deploy user
   - Consider certificate-based SSH

4. **Monitoring**
   - Log all sudo usage
   - Monitor service restarts
   - Alert on unusual patterns

## Implementation Commands

### Create Deploy User
```bash
# On target server
sudo useradd -m -s /bin/bash deploy
sudo mkdir -p /home/deploy/.ssh
sudo cp ~/.ssh/authorized_keys /home/deploy/.ssh/
sudo chown -R deploy:deploy /home/deploy/.ssh
sudo chmod 700 /home/deploy/.ssh
sudo chmod 600 /home/deploy/.ssh/authorized_keys
```

### Configure Sudoers
```bash
# On target server
sudo visudo -f /etc/sudoers.d/disbot-deploy
# Add the restrictive sudoers entry shown above
```

### Update Deploy Script
```bash
# Change USER variable or add DEPLOY_USER
DEPLOY_USER=${DEPLOY_USER:-deploy}
# Use $DEPLOY_USER instead of $USER in SSH commands
```

This plan provides multiple approaches with security analysis to eliminate sudo password prompts during deployment.

## IMPLEMENTED SOLUTION: User Services

**Status:** ✅ Deploy script updated to use `systemctl --user`

### One-Time Server Setup Required

Run these commands on your Fedora server to complete the migration:

```bash
# 1. Create user systemd directory
mkdir -p ~/.config/systemd/user

# 2. Move service file from system to user location
sudo mv /etc/systemd/system/disbot-prod.service ~/.config/systemd/user/
sudo chown $USER:$USER ~/.config/systemd/user/disbot-prod.service

# 3. Enable lingering so service starts at boot (without user login)
sudo loginctl enable-linger $USER

# 4. Reload user systemd and enable the service
systemctl --user daemon-reload
systemctl --user enable disbot-prod.service
systemctl --user start disbot-prod.service

# 5. Verify the service is running
systemctl --user status disbot-prod.service
```

### Verification Steps

After setup, test that deployments work without sudo prompts:

```bash
# From your local machine, run a deployment
./bin/deploy prod your-server-hostname

# Should complete without any password prompts
```

### Service Management Commands (for reference)

```bash
# Check service status
systemctl --user status disbot-prod

# View logs
journalctl --user -u disbot-prod -f

# Manual restart
systemctl --user restart disbot-prod

# Stop service
systemctl --user stop disbot-prod
```

### Benefits of This Implementation

- ✅ **No sudo required** - Eliminates password prompts during deployment
- ✅ **Auto-start at boot** - Lingering ensures service starts without user login
- ✅ **Same systemd functionality** - All systemd features still available
- ✅ **Better security** - Service runs in user scope, not system-wide
- ✅ **No additional dependencies** - Pure systemd solution

## Implementation Summary

### Problem Solved
The original deployment process required multiple interactive sudo password prompts, breaking automation and requiring manual intervention during deployments.

### Solution Approach
Implemented a two-pronged solution combining **user-scope systemd services** with **SSH connection multiplexing**.

**User Services Migration:**
- Migrated from system-wide (`sudo systemctl`) to user-scope (`systemctl --user`) service management
- Eliminates privilege escalation during deployments
- Maintains all systemd functionality while improving security posture

**SSH Connection Optimization:**
- Implemented SSH ControlMaster/ControlPath for connection reuse
- Single authentication establishes persistent connection for all deployment operations
- Automatic cleanup handling prevents connection leaks

### Key Benefits
- **Zero-interaction deployments** - No password prompts or manual intervention required
- **Improved security** - Services run in user scope instead of requiring root privileges
- **Better performance** - SSH multiplexing reduces connection overhead and latency
- **CI/CD ready** - Fully automated deployment process suitable for continuous integration

### Technical Foundation
The solution leverages existing systemd user services capabilities and SSH connection multiplexing features, requiring no additional dependencies or complex tooling. The approach maintains operational simplicity while solving the automation barriers that previously required manual deployment steps.
