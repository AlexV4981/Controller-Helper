"""A conflicting server must fail before creating or recovering settings."""
import pathlib,socket,subprocess,tempfile
repo=pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='studio-port-conflict-') as tmp, socket.socket() as occupied:
 occupied.bind(('127.0.0.1',0));occupied.listen();port=occupied.getsockname()[1];home=pathlib.Path(tmp)
 result=subprocess.run([str(repo/'target/release/controller-studio'),'--background','--home',tmp,'--port',str(port)],capture_output=True,text=True,timeout=10)
 assert result.returncode!=0 and 'already in use' in result.stderr,result
 assert not list(home.iterdir()),'Failed startup changed the settings home before reserving its port'
 print('Port conflict refuses startup before any settings write')
