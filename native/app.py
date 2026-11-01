#!/usr/bin/python3
"""The desktop window. Controller input and saved settings stay in the Rust engine."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
from urllib.parse import urlsplit

VERSION = '0.5.0'


def allowed_navigation(uri, origin):
    try:
        url, local = urlsplit(uri), urlsplit(origin)
        return (url.scheme == local.scheme == 'http' and url.hostname == '127.0.0.1'
                and url.port == local.port and url.username is None and url.password is None)
    except (ValueError, TypeError):
        return False


class Connection:
    def __init__(self, backend, home, port):
        if not 1 <= port <= 65535:
            raise ValueError('Choose a port between 1 and 65535')
        self.backend = Path(backend).resolve()
        self.home = str(Path(home).resolve())
        self.origin = f'http://127.0.0.1:{port}'
        self.port = port
        self.process = None
        self.log = None
        # Never send the loopback session through a desktop HTTP proxy.
        self.http = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def matches(self, health):
        return isinstance(health, dict) and health.get('app') == 'controller-studio' \
            and health.get('version') == VERSION and health.get('home') == self.home

    def health(self):
        try:
            with self.http.open(self.origin + '/health', timeout=1) as response:
                raw = response.read(8193)
                if len(raw) > 8192:
                    return {}
                return json.loads(raw)
        except urllib.error.HTTPError:
            return {}
        except (urllib.error.URLError, TimeoutError, OSError):
            return None
        except (ValueError, UnicodeError):
            return {}

    def ensure_ready(self):
        health = self.health()
        if health is not None:
            if self.matches(health):
                return
            raise RuntimeError('The Controller Studio port is already in use by another or older service. Close the old helper and reopen Studio.')
        if not self.backend.is_file() or not os.access(self.backend, os.X_OK):
            raise RuntimeError('The Controller Studio engine is missing. Reinstall the desktop app and engine together.')
        self.log = tempfile.TemporaryFile(mode='w+b')
        self.process = subprocess.Popen(
            [str(self.backend), '--background', '--home', self.home, '--port', str(self.port)],
            stdin=subprocess.DEVNULL, stdout=self.log, stderr=subprocess.STDOUT, start_new_session=True)
        deadline = time.monotonic() + 12
        while time.monotonic() < deadline:
            health = self.health()
            if self.matches(health):
                return
            if health is not None:
                raise RuntimeError('A different service took the Controller Studio port. It has not been opened.')
            if self.process.poll() is not None:
                self.log.seek(0)
                detail = self.log.read(4096).decode('utf-8', errors='replace')
                raise RuntimeError('The controller engine could not start.\n' + detail)
            time.sleep(.1)
        # Leave the engine alone: it may still be finishing a settings operation.
        raise RuntimeError('The controller engine did not become ready. Close and reopen Studio to retry.')


def main(argv=None):
    parser = argparse.ArgumentParser(description='Controller Studio desktop window')
    parser.add_argument('--backend', type=Path, default=Path(__file__).with_name('controller-studio'))
    parser.add_argument('--home', type=Path, default=Path.home())
    parser.add_argument('--port', type=int, default=17645)
    args = parser.parse_args(argv)
    if os.geteuid() == 0:
        parser.error('Open Controller Studio as your normal desktop user, without sudo.')
    connection = Connection(args.backend, args.home, args.port)
    try:
        import gi
        gi.require_version('Gtk', '3.0')
        gi.require_version('WebKit2', '4.1')
        from gi.repository import Gio, GLib, Gtk, WebKit2
    except (ImportError, ValueError) as error:
        print('Controller Studio needs python3-gi and gir1.2-webkit2-4.1. Install these from Mint’s Software Manager.\n' + str(error), file=sys.stderr)
        return 1

    class Studio(Gtk.Application):
        def __init__(self):
            identity = hashlib.sha256(f'{connection.home}:{args.port}'.encode()).hexdigest()[:24]
            super().__init__(application_id='io.github.AlexV4981.ControllerStudio.i' + identity,
                             flags=Gio.ApplicationFlags.FLAGS_NONE)
            self.window = None
            self.loading = False
            self.web = None
            self.unsaved = False
            self.saving = False

        def do_activate(self):
            if self.window is not None:
                self.window.present()
                return
            self.window = Gtk.ApplicationWindow(application=self)
            self.window.set_title('Controller Studio')
            self.window.set_default_size(1280, 850)
            self.window.set_icon_name('input-gaming')
            self.window.set_wmclass('controller-studio', 'ControllerStudio')
            self.window.connect('destroy', self.closed)
            self.window.connect('delete-event', self.confirm_close)
            self.stack = Gtk.Stack()
            self.window.add(self.stack)
            page = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
            page.set_valign(Gtk.Align.CENTER)
            page.set_halign(Gtk.Align.CENTER)
            page.set_border_width(32)
            self.message = Gtk.Label(label='Opening your controllers…')
            self.message.set_line_wrap(True)
            self.message.set_max_width_chars(72)
            page.pack_start(self.message, False, False, 0)
            self.retry = Gtk.Button(label='Try again')
            self.retry.connect('clicked', lambda _: self.start())
            page.pack_start(self.retry, False, False, 0)
            self.stack.add_named(page, 'status')
            self.window.show_all()
            self.start()

        def closed(self, *_):
            self.window = None
            # The background engine retains queued settings and watches controllers.
            self.quit()

        def confirm_close(self, *_):
            if not (self.unsaved or self.saving):
                return False
            dialog = Gtk.MessageDialog(transient_for=self.window, modal=True,
                message_type=Gtk.MessageType.QUESTION, buttons=Gtk.ButtonsType.NONE,
                text='A settings operation is finishing.' if self.saving else 'Discard unsaved controller changes?')
            dialog.format_secondary_text('Please wait before closing.' if self.saving else 'Saved controls and queued changes are kept. Unsaved edits will be lost.')
            dialog.add_button('Keep open', Gtk.ResponseType.CANCEL)
            if not self.saving:
                dialog.add_button('Discard and close', Gtk.ResponseType.ACCEPT)
            response = dialog.run()
            dialog.destroy()
            return response != Gtk.ResponseType.ACCEPT

        def start(self):
            if self.loading:
                return
            self.loading = True
            self.retry.set_sensitive(False)
            self.message.set_text('Opening your controllers…')
            self.stack.set_visible_child_name('status')

            def work():
                try:
                    connection.ensure_ready()
                    error = None
                except (OSError, RuntimeError) as failure:
                    error = str(failure)
                GLib.idle_add(self.ready, error)
            threading.Thread(target=work, daemon=True).start()

        def ready(self, error):
            self.loading = False
            if self.window is None:
                return False
            self.retry.set_sensitive(True)
            if error:
                self.message.set_text(error)
                return False
            if self.web is None:
                context = WebKit2.WebContext.new_ephemeral()
                context.connect('download-started', lambda _, download: download.cancel())
                self.web = WebKit2.WebView.new_with_context(context)
                settings = self.web.get_settings()
                settings.set_enable_developer_extras(False)
                settings.set_enable_webgl(True)
                self.web.connect('decide-policy', self.policy)
                self.web.connect('permission-request', self.deny_permission)
                self.web.connect('create', lambda *_: None)
                self.web.connect('load-failed', self.load_failed)
                self.web.connect('web-process-terminated', self.web_failed)
                manager = self.web.get_user_content_manager()
                manager.connect('script-message-received::studio', self.script_message)
                manager.register_script_message_handler('studio')
                self.stack.add_named(self.web, 'studio')
                self.web.show()
            self.web.load_uri(connection.origin + '/')
            self.stack.set_visible_child_name('studio')
            return False

        def policy(self, _, decision, kind):
            if kind in (WebKit2.PolicyDecisionType.NAVIGATION_ACTION,
                        WebKit2.PolicyDecisionType.NEW_WINDOW_ACTION):
                uri = decision.get_navigation_action().get_request().get_uri()
                if kind == WebKit2.PolicyDecisionType.NEW_WINDOW_ACTION or not allowed_navigation(uri, connection.origin):
                    decision.ignore()
                    return True
            return False

        def deny_permission(self, _, request):
            request.deny()
            return True

        def script_message(self, _, result):
            if not allowed_navigation(self.web.get_uri(), connection.origin):
                return
            message = result.get_js_value().to_string()
            if message in ('dirty', 'clean', 'busy'):
                self.saving = message == 'busy'
                self.unsaved = message == 'dirty'
            elif message == 'quit':
                self.window.destroy()

        def load_failed(self, _, event, uri, error):
            self.message.set_text('The controller window could not load. Use Try again to reconnect.')
            self.stack.set_visible_child_name('status')
            return True

        def web_failed(self, *_):
            self.message.set_text('The controller window stopped. Your saved controls are kept. Use Try again to reopen it.')
            self.stack.set_visible_child_name('status')

    return Studio().run([sys.argv[0]])


if __name__ == '__main__':
    sys.exit(main())
