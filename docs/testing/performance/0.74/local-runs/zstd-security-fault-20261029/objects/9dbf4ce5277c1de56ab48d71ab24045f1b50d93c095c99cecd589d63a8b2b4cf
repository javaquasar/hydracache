"""Read file allocation metadata without requesting content access or privileges."""
import ctypes as c
from ctypes import wintypes as w
import os


class StandardInfo(c.Structure):
    _fields_ = [('allocation_size', c.c_longlong), ('end_of_file', c.c_longlong),
                ('links', w.DWORD), ('delete_pending', c.c_ubyte), ('directory', c.c_ubyte)]


def query(path):
    if os.name != 'nt':
        raise ValueError('Windows metadata only')
    dll = c.WinDLL('kernel32', use_last_error=True)
    dll.CreateFileW.argtypes = [w.LPCWSTR, w.DWORD, w.DWORD, c.c_void_p, w.DWORD, w.DWORD, w.HANDLE]
    dll.CreateFileW.restype = w.HANDLE
    dll.GetFileInformationByHandleEx.argtypes = [w.HANDLE, c.c_int, c.c_void_p, w.DWORD]
    dll.GetFileInformationByHandleEx.restype = w.BOOL
    dll.CloseHandle.argtypes = [w.HANDLE]
    dll.CloseHandle.restype = w.BOOL
    # DesiredAccess=0; share all; OPEN_EXISTING. No read/write content, settings,
    # privilege changes, deletion, replacement, volume locking or retry.
    handle = dll.CreateFileW(str(path), 0, 7, None, 3, 0, None)
    if handle == c.c_void_p(-1).value:
        return dict(path=str(path), allocation_bytes=None, open_winerror=c.get_last_error())
    result = dict(path=str(path))
    try:
        info = StandardInfo()
        if dll.GetFileInformationByHandleEx(handle, 1, c.byref(info), c.sizeof(info)):
            result.update(allocation_bytes=info.allocation_size, logical_bytes=info.end_of_file,
                          links=info.links, delete_pending=bool(info.delete_pending),
                          directory=bool(info.directory), query_winerror=None)
        else:
            result.update(allocation_bytes=None, query_winerror=c.get_last_error())
    finally:
        result['close_winerror'] = None if dll.CloseHandle(handle) else c.get_last_error()
    return result
