"""Build SkyOS initrd.tar with FHS directory structure."""
import tarfile, os, sys, io

def build_initrd(root_dir: str, output_path: str):
    coreutils_bins = [
        'ls', 'cat', 'mkdir', 'rm', 'cp', 'mv',
        'ps', 'clear', 'uname', 'printenv', 'sleep', 'yes',
        'rmdir', 'touch', 'hostname', 'which', 'env', 'echo', 'head', 'tail', 'wc', 'grep', 'ln', 'chmod',
        'printf', 'sort', 'uniq', 'uptime',
        'ping', 'nslookup', 'wget', 'ifconfig', 'netstat', 'telnet',
        'beep', 'dd', 'blkid', 'fdisk', 'df', 'du',
    ]

    binaries = {
        'bin/init':          'init',
        'bin/sargash':       'sargash',
        'bin/sash':          'sash',
        'bin/svc':           'svc',
        'bin/vahid':         'vahid',
        'bin/skyedit':       'skyedit',
        'bin/skypkg':        'skypkg',
        'bin/login-manager': 'login-manager',
        'bin/login':         'login',
        'bin/passwd':        'passwd',
        'bin/skybuild':      'skybuild',
        'bin/setup':         'setup',
        'bin/mkfs.ext2':     'mkfs_ext2',
        'bin/mkfs.fat':      'mkfs_fat',
        'bin/sarga-term':    'sarga-term',
        'bin/calculator':    'calculator',
        'bin/clock':         'clock',
        'bin/calendar':      'calendar',
        'bin/notes':         'notes',
        'bin/paint':         'paint',
        'bin/search':        'search',
        'bin/tasks':         'tasks',
        'bin/archive':       'archive',
        'bin/sysinfo':       'sysinfo',
        'bin/sysmon':        'sysmon',
        'bin/nettools':      'nettools',
        'bin/ade':           'ade',
        'bin/aicli':         'aicli',
        'bin/skystore':      'skystore',
        'bin/spkg':          'spkg',
    }
    for b in coreutils_bins:
        binaries[f'bin/{b}'] = b

    symlinks = {
        'sbin/init':          '../bin/init',
        'sbin/vahid':         '../bin/vahid',
        'sbin/svc':           '../bin/svc',
        'sbin/skyedit':       '../bin/skyedit',
    }

    empty_dirs = [
        'dev',
        'proc',
        'tmp',
        'usr/lib',
        'usr/share',
        'usr/include',
        'var/log',
        'var/cache',
        'var/spool',
        'var/skypkg',
        'home/root',
        'mnt/cdrom',
        'mnt/usb',
    ]

    config_files = {
        'etc/init.toml': None,
        'etc/fstab': None,
        'etc/hostname': None,
        'etc/passwd': None,
        'etc/shadow': None,
        'etc/group': None,
    }

    if os.path.exists(output_path):
        os.remove(output_path)

    # Binary locations to search (from cargo build output)
    search_dirs = [
        os.path.join(root_dir, 'target', 'x86_64-sarga', 'release'),
        os.path.join(root_dir, 'target', 'x86_64-sarga', 'debug'),
        root_dir,
    ]

    with tarfile.open(output_path, 'w') as tar:
        # Add regular binaries
        for arcname, binary in binaries.items():
            found = False
            for d in search_dirs:
                full_path = os.path.join(d, binary)
                if os.path.exists(full_path):
                    tar.add(full_path, arcname=arcname)
                    print(f'  {arcname} ({os.path.getsize(full_path)} bytes)')
                    found = True
                    break
            if not found:
                print(f'  WARNING: {binary} not found in search paths')

        # Add config files
        init_toml_data = read_config(root_dir, 'etc/init.toml') or INIT_TOML_CONTENT
        config_data = {
            'etc/init.toml': init_toml_data,
            'etc/fstab': FSTAB_CONTENT,
            'etc/hostname': HOSTNAME_CONTENT,
            'etc/passwd': PASSWD_CONTENT,
            'etc/shadow': SHADOW_CONTENT,
            'etc/group': GROUP_CONTENT,
        }
        for arcname, data in config_data.items():
            info = tarfile.TarInfo(name=arcname)
            info.type = tarfile.REGTYPE
            encoded = data.encode('utf-8')
            info.size = len(encoded)
            tar.addfile(info, io.BytesIO(encoded))
            print(f'  {arcname} ({len(encoded)} bytes)')

        # Add symlinks
        for arcname, target in symlinks.items():
            info = tarfile.TarInfo(name=arcname)
            info.type = tarfile.SYMTYPE
            info.linkname = target
            tar.addfile(info)
            print(f'  {arcname} -> {target}')

        # Add empty directories
        for dirname in empty_dirs:
            info = tarfile.TarInfo(name=dirname)
            info.type = tarfile.DIRTYPE
            info.mode = 0o755
            tar.addfile(info)
            print(f'  {dirname}/')

    size = os.path.getsize(output_path)
    print(f'\ninitrd.tar: {size} bytes ({size/1024:.1f} KB)')

def read_config(root_dir, path):
    full = os.path.join(root_dir, path)
    if os.path.exists(full):
        with open(full, 'r') as f:
            return f.read()
    return ''

FSTAB_CONTENT = """# /etc/fstab - filesystem mount table
# <source>  <mountpoint>  <fstype>  <options>  <dump>  <pass>
tmpfs       /tmp          tmpfs     defaults   0       0
tmpfs       /var/log      tmpfs     defaults   0       0
tmpfs       /var/cache    tmpfs     defaults   0       0
tmpfs       /home         tmpfs     defaults   0       0
"""

HOSTNAME_CONTENT = "skyos\n"

PASSWD_CONTENT = """root:x:0:0:root:/home/root:/bin/sash
"""

SHADOW_CONTENT = """root:$6$rounds=5000$usesalt$:12000:0:99999:7:::
"""

GROUP_CONTENT = """root:x:0:root
wheel:x:1:root
users:x:100:
"""

INIT_TOML_CONTENT = """hostname = "skyos"

[[service]]
name = "login-manager"
exec = "/bin/login-manager"
respawn = true

[[service]]
name = "svc"
exec = "/bin/svc"
respawn = true
"""

if __name__ == '__main__':
    root = sys.argv[1] if len(sys.argv) > 1 else '.'
    output = os.path.join(root, 'initrd.tar')
    build_initrd(root, output)
