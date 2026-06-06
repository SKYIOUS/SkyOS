import tarfile, os

root = r'C:\Users\nanda\Desktop\Github\SkyOS\staging'
tar_path = r'C:\Users\nanda\Desktop\Github\SkyOS\initrd.tar'
if os.path.exists(tar_path):
    os.remove(tar_path)

with tarfile.open(tar_path, 'w') as tar:
    # Add root directory
    dir_info = tarfile.TarInfo(name=".")
    dir_info.type = tarfile.DIRTYPE
    dir_info.mode = 0o755
    tar.addfile(dir_info)

    for dirpath, dirnames, filenames in os.walk(root):
        # Add directory entry
        rel_dir = os.path.relpath(dirpath, root).replace('\\', '/')
        if rel_dir != '.':
            dir_info = tarfile.TarInfo(name=rel_dir)
            dir_info.type = tarfile.DIRTYPE
            dir_info.mode = 0o755
            tar.addfile(dir_info)

        for f in filenames:
            full = os.path.join(dirpath, f)
            arcname = os.path.relpath(full, root).replace('\\', '/')
            tar.add(full, arcname=arcname)

with tarfile.open(tar_path, 'r') as tar:
    for m in tar.getmembers():
        kind = 'dir' if m.isdir() else 'file'
        print('  {} ({})'.format(m.name, kind))
print('Created {} ({} bytes)'.format(tar_path, os.path.getsize(tar_path)))
