"""Narrow X11 event/screenshot test helper. Callers must pass their owned window ID."""
import ctypes as C
import ctypes.util
import time

class XKeyEvent(C.Structure):
    _fields_=[('type',C.c_int),('serial',C.c_ulong),('send_event',C.c_int),('display',C.c_void_p),('window',C.c_ulong),('root',C.c_ulong),('subwindow',C.c_ulong),('time',C.c_ulong),('x',C.c_int),('y',C.c_int),('x_root',C.c_int),('y_root',C.c_int),('state',C.c_uint),('keycode',C.c_uint),('same_screen',C.c_int)]
class XEvent(C.Union):
    _fields_=[('key',XKeyEvent),('pad',C.c_long*24)]
class XWindowAttributes(C.Structure):
    # ABI verified against installed /usr/include/X11/Xlib.h, not window contents.
    _fields_=[('x',C.c_int),('y',C.c_int),('width',C.c_int),('height',C.c_int),('border_width',C.c_int),('depth',C.c_int),('visual',C.c_void_p),('root',C.c_ulong),('c_class',C.c_int),('bit_gravity',C.c_int),('win_gravity',C.c_int),('backing_store',C.c_int),('backing_planes',C.c_ulong),('backing_pixel',C.c_ulong),('save_under',C.c_int),('colormap',C.c_ulong),('map_installed',C.c_int),('map_state',C.c_int),('all_event_masks',C.c_long),('your_event_mask',C.c_long),('do_not_propagate_mask',C.c_long),('override_redirect',C.c_int),('screen',C.c_void_p)]
class XErrorEvent(C.Structure):
    _fields_=[('type',C.c_int),('display',C.c_void_p),('resourceid',C.c_ulong),('serial',C.c_ulong),('error_code',C.c_ubyte),('request_code',C.c_ubyte),('minor_code',C.c_ubyte)]
class X11:
    def __init__(self):
        self.lib=C.CDLL(ctypes.util.find_library('X11'))
        self.lib.XOpenDisplay.restype=C.c_void_p;self.lib.XOpenDisplay.argtypes=[C.c_char_p]
        self.display=self.lib.XOpenDisplay(None)
        if not self.display:raise RuntimeError('No accessible X11 display; native journey NOT RUN')
        # Process-local test error handling: propagate errors through Python's
        # finally cleanup instead of Xlib's default fatal process exit. No server
        # configuration or another process's error policy is changed.
        self.errors=[]
        callback=C.CFUNCTYPE(C.c_int,C.c_void_p,C.POINTER(XErrorEvent))
        def error(display,event):
            if len(self.errors)<8:self.errors.append((event.contents.error_code,event.contents.request_code))
            return 0
        self.error_handler=callback(error)
        self.lib.XSetErrorHandler.argtypes=[C.c_void_p];self.lib.XSetErrorHandler.restype=C.c_void_p
        self.prior_handler=self.lib.XSetErrorHandler(C.cast(self.error_handler,C.c_void_p))
        self.lib.XSync.argtypes=[C.c_void_p,C.c_int]

        self.lib.XStringToKeysym.restype=C.c_ulong;self.lib.XStringToKeysym.argtypes=[C.c_char_p]
        self.lib.XKeysymToKeycode.restype=C.c_uint;self.lib.XKeysymToKeycode.argtypes=[C.c_void_p,C.c_ulong]
        self.lib.XSendEvent.argtypes=[C.c_void_p,C.c_ulong,C.c_int,C.c_long,C.POINTER(XEvent)]
        self.lib.XFlush.argtypes=[C.c_void_p];self.lib.XCloseDisplay.argtypes=[C.c_void_p]
    def key(self,window,name,state=0):
        keycode=self.lib.XKeysymToKeycode(self.display,self.lib.XStringToKeysym(name.encode()))
        if not keycode:raise RuntimeError('Unavailable X11 test key '+name)
        for kind in [2,3]:
            e=XEvent();e.key=XKeyEvent(kind,0,1,self.display,window,0,0,0,1,1,1,1,state,keycode,1)
            if not self.lib.XSendEvent(self.display,window,False,1<<(kind-2),C.byref(e)):raise RuntimeError('Owned-window XSendEvent failed')
        self.lib.XFlush(self.display)
    def focus(self):
        window=C.c_ulong();revert=C.c_int()
        self.lib.XGetInputFocus.argtypes=[C.c_void_p,C.POINTER(C.c_ulong),C.POINTER(C.c_int)]
        self.lib.XSync.argtypes=[C.c_void_p,C.c_int]
        self.lib.XSync(self.display,False)
        self.lib.XGetInputFocus(self.display,C.byref(window),C.byref(revert))
        return window.value
    def sink(self,native):
        if self.focus()!=native:raise RuntimeError('Owned native window is not focused; will not steal user focus')
        self.lib.XDefaultRootWindow.argtypes=[C.c_void_p];self.lib.XDefaultRootWindow.restype=C.c_ulong
        self.lib.XCreateSimpleWindow.argtypes=[C.c_void_p,C.c_ulong,C.c_int,C.c_int,C.c_uint,C.c_uint,C.c_uint,C.c_ulong,C.c_ulong]
        self.lib.XCreateSimpleWindow.restype=C.c_ulong
        sink=self.lib.XCreateSimpleWindow(self.display,self.lib.XDefaultRootWindow(self.display),30,30,240,60,0,0,0x222222)
        self.lib.XStoreName.argtypes=[C.c_void_p,C.c_ulong,C.c_char_p]
        self.lib.XStoreName(self.display,sink,b'namtab test-owned neutral focus sink')
        self.restore(sink)
        return sink
    def focus_owned(self,target,owned):
        if target not in owned or self.focus() not in owned:raise RuntimeError('Focus transfer refused: source/target must both be test-owned')
        self.lib.XGetWindowAttributes.argtypes=[C.c_void_p,C.c_ulong,C.POINTER(XWindowAttributes)]
        deadline=time.monotonic()+2
        while True:
            attributes=XWindowAttributes()
            found=self.lib.XGetWindowAttributes(self.display,target,C.byref(attributes))
            self.sync()
            if found and attributes.map_state==2:break  # IsViewable, not merely XMap queued.
            if time.monotonic()>deadline:raise RuntimeError('Owned focus target never became viewable; no focus request sent')
            time.sleep(.01)
        self.lib.XSetInputFocus.argtypes=[C.c_void_p,C.c_ulong,C.c_int,C.c_ulong]
        self.lib.XSetInputFocus(self.display,target,2,0)
        self.sync()
        if self.focus()!=target:raise RuntimeError('Owned focus transition not established')
    def destroy(self,window):
        self.lib.XDestroyWindow.argtypes=[C.c_void_p,C.c_ulong]
        self.lib.XDestroyWindow(self.display,window);self.sync()
    def sync(self):
        self.lib.XSync(self.display,False)
        if self.errors:
            errors=self.errors[:];self.errors.clear()
            raise RuntimeError('Owned X11 fixture request failed (error,request): '+str(errors))
    def close(self):
        try:self.sync()
        finally:
            self.lib.XCloseDisplay(self.display)
            self.lib.XSetErrorHandler(self.prior_handler)

    def find(self,pid):
        lib=self.lib
        lib.XDefaultRootWindow.argtypes=[C.c_void_p];lib.XDefaultRootWindow.restype=C.c_ulong
        lib.XInternAtom.argtypes=[C.c_void_p,C.c_char_p,C.c_int];lib.XInternAtom.restype=C.c_ulong
        lib.XQueryTree.argtypes=[C.c_void_p,C.c_ulong,C.POINTER(C.c_ulong),C.POINTER(C.c_ulong),C.POINTER(C.POINTER(C.c_ulong)),C.POINTER(C.c_uint)]
        lib.XGetWindowProperty.argtypes=[C.c_void_p,C.c_ulong,C.c_ulong,C.c_long,C.c_long,C.c_int,C.c_ulong,C.POINTER(C.c_ulong),C.POINTER(C.c_int),C.POINTER(C.c_ulong),C.POINTER(C.c_ulong),C.POINTER(C.POINTER(C.c_ubyte))]
        lib.XFree.argtypes=[C.c_void_p]
        atom=lib.XInternAtom(self.display,b'_NET_WM_PID',True)
        pending=[lib.XDefaultRootWindow(self.display)];count=0
        while pending and count<1024:
            window=pending.pop();count+=1
            typ=C.c_ulong();fmt=C.c_int();items=C.c_ulong();after=C.c_ulong();data=C.POINTER(C.c_ubyte)()
            status=lib.XGetWindowProperty(self.display,window,atom,0,1,False,0,C.byref(typ),C.byref(fmt),C.byref(items),C.byref(after),C.byref(data))
            found=status==0 and fmt.value==32 and items.value==1 and C.cast(data,C.POINTER(C.c_ulong))[0]==pid
            if data:lib.XFree(data)
            if found:return window
            root=C.c_ulong();parent=C.c_ulong();children=C.POINTER(C.c_ulong)();n=C.c_uint()
            if lib.XQueryTree(self.display,window,C.byref(root),C.byref(parent),C.byref(children),C.byref(n)):
                pending.extend(children[i] for i in range(n.value))
                if children:lib.XFree(children)
        return None
    def resize(self,window,width,height):
        self.lib.XResizeWindow.argtypes=[C.c_void_p,C.c_ulong,C.c_uint,C.c_uint]
        self.lib.XResizeWindow(self.display,window,width,height);self.lib.XFlush(self.display)
    def focus_event(self,window,focused):
        # Synthetic focus notification ONLY to our test window. Does not call
        # XSetInputFocus or alter the user's actual focused app.
        class Focus(C.Structure):
            _fields_=[('type',C.c_int),('serial',C.c_ulong),('send_event',C.c_int),('display',C.c_void_p),('window',C.c_ulong),('mode',C.c_int),('detail',C.c_int)]
        f=Focus(9 if focused else 10,0,1,self.display,window,0,3)
        event=XEvent();C.memmove(C.byref(event),C.byref(f),C.sizeof(f))
        self.lib.XSendEvent(self.display,window,False,1<<21,C.byref(event));self.lib.XFlush(self.display)
    def minimize(self,window):
        self.lib.XDefaultScreen.argtypes=[C.c_void_p];self.lib.XDefaultScreen.restype=C.c_int
        self.lib.XIconifyWindow.argtypes=[C.c_void_p,C.c_ulong,C.c_int]
        self.lib.XIconifyWindow(self.display,window,self.lib.XDefaultScreen(self.display));self.lib.XFlush(self.display)
    def restore(self,window):
        self.lib.XMapWindow.argtypes=[C.c_void_p,C.c_ulong]
        self.lib.XMapWindow(self.display,window);self.lib.XFlush(self.display)
