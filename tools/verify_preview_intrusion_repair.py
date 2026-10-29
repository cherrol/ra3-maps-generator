"""Independent NPZ/PNG readback for the reviewed regional intrusion repairs."""
import ast,hashlib,json,struct,sys,zipfile,zlib
from collections import Counter,deque
from pathlib import Path
COLORS=[bytes(c) for c in [(53,116,150),(104,151,89),(160,178,99),(202,180,119),(162,128,103),(218,221,211)]]
DARK=[bytes(int(v*.77) for v in c) for c in COLORS]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def decode(b):
 offset=10 if b[6]==1 else 12;size=int.from_bytes(b[8:offset],'little');meta=ast.literal_eval(b[offset:offset+size].decode());assert not meta['fortran_order'];return meta,memoryview(b)[offset+size:]
def arrays(p):
 with zipfile.ZipFile(p) as z:
  assert z.testzip() is None;return {n.removesuffix('.npy'):decode(z.read(n)) for n in z.namelist() if n.endswith('.npy')}
def png(p):
 b=p.read_bytes();assert b[:8]==b'\x89PNG\r\n\x1a\n';pos=8;packed=[];w=h=None
 while pos<len(b):
  n=int.from_bytes(b[pos:pos+4],'big');kind=b[pos+4:pos+8];body=b[pos+8:pos+8+n];crc=int.from_bytes(b[pos+8+n:pos+12+n],'big');assert zlib.crc32(kind+body)&0xffffffff==crc
  if kind==b'IHDR':w,h,depth,color,c,f,i=struct.unpack('>IIBBBBB',body);assert (depth,color,c,f,i)==(8,2,0,0,0)
  if kind==b'IDAT':packed.append(body)
  pos+=n+12
 raw=zlib.decompress(b''.join(packed));stride=w*3+1;assert len(raw)==stride*h and all(raw[y*stride]==0 for y in range(h));return w,h,raw,stride

def waterbodies(a,W,H,b):
 seen=bytearray(W*H);out=[]
 for y in range(b,H-b):
  for x in range(b,W-b):
   start=y*W+x
   if a[start] or seen[start]:continue
   cells=[];q=deque([start]);seen[start]=1;votes=Counter();opened=False
   while q:
    i=q.popleft();iy,ix=divmod(i,W);cells.append(i);opened|=ix in [b,W-b-1] or iy in [b,H-b-1]
    for k in (i-1,i+1,i-W,i+W):
     yy,xx=divmod(k,W)
     if not(b<=xx<W-b and b<=yy<H-b):continue
     if a[k]:votes[a[k]]+=1
     elif not seen[k]:seen[k]=1;q.append(k)
   out.append({'water_cells':len(cells),'bounds':[min(i%W for i in cells),min(i//W for i in cells),max(i%W for i in cells)+1,max(i//W for i in cells)+1],'open':opened,'contacts':dict(votes)})
 return out


# Production rendering remains in Rust; this independent verifier uses only stdlib.
ROOT=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else Path.cwd()
OUTPUT=Path(sys.argv[2]) if len(sys.argv)>2 else Path('/tmp/preview-intrusion-readback.json')
results=[]
for p in sorted((ROOT/'maps').glob('*/data/terrain-preview/intrusion-repair-2026-10-30.json')):
 d=p.parent;name=p.parts[-4];m=d.parents[1];r=json.loads((d/'report.json').read_text());v=json.loads(p.read_text());old=arrays(d/v['baseline']/'terrain.npz');a=arrays(d/'terrain.npz');L=a['layers'][1];B=old['layers'][1];errs=[]
 def check(b,s):
  if not b:errs.append(s)
 check(a.keys()==old.keys(),'array inventory');check(r['game_cells']==[800,800] and r['border_cells']==20,'frame')
 for k in a:
  if k not in ['layers','game_heights','water_mask']:check(a[k][1]==old[k][1],f'protected array {k}')
 changed=[i for i in range(840*840) if L[i]!=B[i]];check(changed==v['changed_full_grid_indices'],'exact scope indices')
 allowed=set()
 for site in v['reviewed_sites']:
  for merge in site['merges']:allowed.update(merge['changed_full_grid_indices'])
 for band in v['shore_reconciliation']:allowed.update(band['changed_full_grid_indices'])
 for fragment in v['detached_fragment_merges']:allowed.update(fragment['changed_full_grid_indices'])
 check(set(changed)<=allowed,'outside declared scope');check(all(20<=i%840<820 and 20<=i//840<820 for i in changed),'border changed')
 H=a['game_heights'][1].cast('f');M=a['water_mask'][1];gh=r['game_layer_heights'];check(all(0<=l<=5 and bool(M[i])==(l==0) and H[i]==(130 if l==0 else gh[l-1]) for i,l in enumerate(L)),'height/water consistency')
 check(v['terrain_crc32']==zlib.crc32((d/'terrain.npz').read_bytes())&0xffffffff,'repair checksum')
 sources=[]
 for file in r['final_images']:
  ip=m/file;w,h,rgb,stride=png(ip);check((w,h)==(1104,1002),'image dimensions '+file);check(zlib.crc32(ip.read_bytes())&0xffffffff==r['image_crc32'][file],'image crc '+file);sources.append({'file':file,'sha256':sha(ip),'size':[w,h]})
 w,h,rgb,stride=png(m/'preview.png');mismatch=0
 for y in range(800):
  start=(112+y)*stride+1+28*3
  for x in range(800):
   i=(y+20)*840+x+20;l=L[i];c=COLORS[0] if l==0 else DARK[l] if any(L[k] and L[k]!=l for k in [i-1,i+1,i-840,i+840]) else COLORS[l]
   mismatch+=rgb[start+x*3:start+x*3+3]!=c
 check(mismatch==0,'preview pixel mismatch');bodies=waterbodies(L,840,840,20);closed=[]
 for b in bodies:
  if not b['open']:check(len(b['contacts'])==1,'closed shore multi-tier');closed.append(b)
 # Every retained closed body must retain the owner of the old component at overlapping cells.
 oldlabels={};seen=set();oldowners={};label=0
 from collections import deque,Counter
 for i in range(20*840+20,820*840):
  if not(20<=i%840<820) or B[i] or i in seen:continue
  label+=1;q=deque([i]);seen.add(i);votes=Counter()
  while q:
   k=q.popleft();oldlabels[k]=label
   for j in [k-1,k+1,k-840,k+840]:
    if not(20<=j%840<820 and 20<=j//840<820):continue
    if B[j]:votes[B[j]]+=1
    elif j not in seen:seen.add(j);q.append(j)
  oldowners[label]=max(votes,key=votes.get)
 for b in closed:
  x0,y0,x1,y1=b['bounds'];overlap={oldlabels[y*840+x] for y in range(y0,y1) for x in range(x0,x1) if L[y*840+x]==0 and y*840+x in oldlabels}
  check(all(oldowners[label] in b['contacts'] for label in overlap),'prior water owner changed')
 # Every enclosed shore must reach retained land of the same owner tier.
 land_component={};anchors={};component_id=0
 for b in closed:
  x0,y0,x1,y1=b['bounds'];shore_cells=set()
  for y in range(y0,y1):
   for x in range(x0,x1):
    k=y*840+x
    if L[k]==0:
     shore_cells.update(j for j in [k-1,k+1,k-840,k+840] if 20<=j%840<820 and 20<=j//840<820 and L[j])
  for start in shore_cells:
   if start in land_component:continue
   component_id+=1;q=deque([start]);land_component[start]=component_id;anchor=False
   while q:
    k=q.popleft();anchor|=L[k]==B[k]
    for j in [k-1,k+1,k-840,k+840]:
     if 20<=j%840<820 and 20<=j//840<820 and L[j]==L[start] and j not in land_component:
      land_component[j]=component_id;q.append(j)
   anchors[component_id]=anchor
  anchored=all(anchors[land_component[k]] for k in shore_cells)
  check(anchored,'shore disconnected from retained owner land');b['connected_to_unchanged_owner_land']=anchored
 results.append({'map_id':name,'terrain_sha256':sha(d/'terrain.npz'),'changed_cells':len(changed),'changed_water_cells':sum((L[i]==0)!=(B[i]==0) for i in changed),'images':sources,'pixels_checked':640000,'pixel_mismatches':mismatch,'closed_waterbodies':closed,'waterbodies':bodies,'errors':errs,'independent_checks_passed':not errs})
 print(name,len(changed),'cells',errs,flush=True)
assert len(results)==11
out={'date':'2026-10-30','maps':results,'maps_checked':11,'images_checked':sum(len(v['images']) for v in results),'pixels_checked':sum(v['pixels_checked'] for v in results),'all_independent_checks_passed':all(not v['errors'] for v in results)}
OUTPUT.parent.mkdir(parents=True,exist_ok=True)
OUTPUT.write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
assert out['all_independent_checks_passed']
