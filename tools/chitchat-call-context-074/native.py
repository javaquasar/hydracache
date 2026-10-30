"""Enter the current pinned/derived owner after exact source verification."""
import runpy
import native_codegen as g

if __name__=='__main__':
    path=g.materialize()
    runpy.run_path(str(path),run_name='__main__')
