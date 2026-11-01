"""Small X11 test fixture. It emulates window metadata, never game execution."""
import ctypes as c,os,sys
x=c.CDLL('libX11.so.6');D=c.c_void_p;W=c.c_ulong
x.XOpenDisplay.argtypes=[c.c_char_p];x.XOpenDisplay.restype=D
d=x.XOpenDisplay(None)
if not d:raise SystemExit('No X11 test display')
x.XDefaultRootWindow.argtypes=[D];x.XDefaultRootWindow.restype=W
x.XCreateSimpleWindow.argtypes=[D,W,c.c_int,c.c_int,c.c_uint,c.c_uint,c.c_uint,W,W];x.XCreateSimpleWindow.restype=W
w=x.XCreateSimpleWindow(d,x.XDefaultRootWindow(d),30,30,420,80,0,0,0x22382a)
x.XInternAtom.argtypes=[D,c.c_char_p,c.c_int];x.XInternAtom.restype=W
x.XChangeProperty.argtypes=[D,W,W,W,c.c_int,c.c_int,c.c_void_p,c.c_int]
pid=W(os.getpid());x.XChangeProperty(d,w,x.XInternAtom(d,b'_NET_WM_PID',0),6,32,0,c.byref(pid),1)
class Hint(c.Structure):_fields_=[('res_name',c.c_char_p),('res_class',c.c_char_p)]
x.XSetClassHint.argtypes=[D,W,c.POINTER(Hint)];x.XSetClassHint(d,w,c.byref(Hint(b'dolphin-emu',b'dolphin-emu')))
x.XStoreName.argtypes=[D,W,c.c_char_p];x.XMapWindow.argtypes=[D,W];x.XFlush.argtypes=[D]
c.CDLL(None).prctl(15,b'dolphin-emu',0,0,0)
x.XStoreName(d,w,b'Controller Studio TEST | SMNE01 | New Super Mario Bros. Wii');x.XMapWindow(d,w);x.XFlush(d);print('ready',flush=True)
for line in sys.stdin:
 title=line.strip()
 if title=='QUIT':break
 x.XStoreName(d,w,title.encode());x.XFlush(d);print('title updated',flush=True)
x.XDestroyWindow.argtypes=[D,W];x.XDestroyWindow(d,w);x.XCloseDisplay.argtypes=[D];x.XCloseDisplay(d)
