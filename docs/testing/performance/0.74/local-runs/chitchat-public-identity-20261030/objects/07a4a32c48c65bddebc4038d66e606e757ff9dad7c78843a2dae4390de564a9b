"""One public unauthenticated metadata GET; never publication/ownership authority."""
import datetime,json,urllib.error,urllib.request
import contract as s
URL='https://crates.io/api/v1/crates/hydra-chitchat'

def main():
    out=s.BASE/'public-registry';out.mkdir(exist_ok=False)
    request=urllib.request.Request(URL,headers={'User-Agent':'HydraCache-0.74-readonly (https://github.com/javaquasar/hydracache)','Accept':'application/json'})
    result=dict(url=URL,observed_at_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),retry=False,
        name_reserved=False,ownership_proven=False,publication_authorized=False,root_adopted=False)
    try:
        try:response=urllib.request.urlopen(request,timeout=30)
        except urllib.error.HTTPError as error:response=error
        with response:
            raw=response.read(512*1024+1);s.c.need(len(raw)<=512*1024,'bounded public response body')
            (out/'response.body').write_bytes(raw)
            result.update(http_status=response.code,headers={k:response.headers[k] for k in ('Date','ETag','Content-Type') if k in response.headers},response_sha256=s.c.sha(raw),response_bytes=len(raw),response_received=True)
    except Exception as error:result.update(response_received=False,first_access_error=dict(type=type(error).__name__,reason=str(error)))
    s.c.write(out,'response.json',result);print(json.dumps(result))

if __name__=='__main__':main()
