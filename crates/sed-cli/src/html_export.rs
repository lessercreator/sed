use anyhow::Result;
use sed_sdk::SedDocument;

pub fn export_html(file: &str, output: &str, level: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let info = doc.info()?;
    let rooms = sed_sdk::geometry::get_room_geometry(&doc, level)?;

    let placements = doc.query_params(
        "SELECT pt.tag, pt.category, pt.domain, p.x, p.y, p.cfm, COALESCE(p.instance_tag,'') as itag, COALESCE(s.name,'') as room, pt.manufacturer, pt.model
         FROM placements p JOIN product_types pt ON p.product_type_id = pt.id
         LEFT JOIN spaces s ON p.space_id = s.id
         WHERE p.level = ?1 AND p.x IS NOT NULL ORDER BY pt.tag",
        &[&level as &dyn rusqlite::types::ToSql],
    )?;

    let segments = doc.query_params(
        "SELECT n1.x as x1, n1.y as y1, n2.x as x2, n2.y as y2, seg.diameter_m
         FROM segments seg JOIN nodes n1 ON seg.from_node_id = n1.id JOIN nodes n2 ON seg.to_node_id = n2.id
         WHERE n1.level = ?1 AND n1.x IS NOT NULL AND n2.x IS NOT NULL",
        &[&level as &dyn rusqlite::types::ToSql],
    )?;

    let nodes = doc.query_params(
        "SELECT n.x, n.y, n.node_type, COALESCE(n.fitting_type,'') as ft, COALESCE(n.size_description,'') as sd
         FROM nodes n JOIN systems sys ON n.system_id = sys.id WHERE n.level = ?1 AND n.x IS NOT NULL",
        &[&level as &dyn rusqlite::types::ToSql],
    )?;

    // Build JSON data
    let mut rooms_json = Vec::new();
    for r in &rooms {
        let verts: Vec<String> = r.vertices.iter().map(|v| format!("{{\"x\":{},\"y\":{}}}", v.x, v.y)).collect();
        rooms_json.push(format!(
            "{{\"tag\":\"{}\",\"name\":\"{}\",\"scope\":\"{}\",\"vertices\":[{}]}}",
            esc(&r.tag), esc(&r.name), esc(&r.scope), verts.join(",")
        ));
    }

    let mut place_json = Vec::new();
    for p in &placements {
        place_json.push(format!(
            "{{\"tag\":\"{}\",\"cat\":\"{}\",\"dom\":\"{}\",\"x\":{},\"y\":{},\"cfm\":{},\"itag\":\"{}\",\"room\":\"{}\",\"mfr\":\"{}\",\"model\":\"{}\"}}",
            esc(&p[0].1), esc(&p[1].1), esc(&p[2].1),
            &p[3].1, &p[4].1,
            if p[5].1 == "NULL" { "null" } else { &p[5].1 },
            esc(&p[6].1), esc(&p[7].1), esc(&p[8].1), esc(&p[9].1)
        ));
    }

    let mut seg_json = Vec::new();
    for s in &segments {
        let d: f64 = s[4].1.parse().unwrap_or(0.2);
        seg_json.push(format!(
            "{{\"x1\":{},\"y1\":{},\"x2\":{},\"y2\":{},\"d\":{:.1}}}",
            &s[0].1, &s[1].1, &s[2].1, &s[3].1, d / 0.0254
        ));
    }

    let mut node_json = Vec::new();
    for n in &nodes {
        node_json.push(format!(
            "{{\"x\":{},\"y\":{},\"type\":\"{}\",\"fitting\":\"{}\",\"size\":\"{}\"}}",
            &n[0].1, &n[1].1, esc(&n[2].1), esc(&n[3].1), esc(&n[4].1)
        ));
    }

    let html = format!(r##"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{project} — {level}</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{background:#1a1a1e;overflow:hidden;font-family:'Segoe UI',system-ui,sans-serif}}
canvas{{position:absolute;top:0;left:0;cursor:grab}}
#props{{display:none;position:fixed;right:16px;top:16px;width:320px;background:#ffffffe8;border:1px solid #999;border-radius:6px;padding:16px;font-size:13px;max-height:80vh;overflow-y:auto;box-shadow:0 4px 24px #0004}}
#props h3{{font-size:15px;margin-bottom:8px;display:flex;justify-content:space-between}}
#props .close{{cursor:pointer;color:#999}}
#props .close:hover{{color:#000}}
.pr{{display:flex;justify-content:space-between;padding:2px 0;border-bottom:1px solid #eee}}
.pr .k{{color:#666}}.pr .v{{color:#000;font-weight:500;text-align:right;max-width:180px;word-break:break-word}}
#info{{position:fixed;left:16px;bottom:16px;color:#888;font-size:11px}}
</style></head><body>
<canvas id="c"></canvas>
<div id="props"></div>
<div id="info">{project} #{number} — {level} | Pan: drag | Zoom: scroll | Click: inspect</div>
<script>
const DATA = {{
  rooms: [{rooms}],
  placements: [{placements}],
  segments: [{segments}],
  nodes: [{nodes}],
  project: "{project}",
  number: "{number}",
  level: "{level}",
  sheet: "{sheet_num}"
}};

const c = document.getElementById('c');
const ctx = c.getContext('2d');
let W, H, vx=0, vy=0, vs=1, dragging=false, dsx=0, dsy=0;

// Sheet constants (mm)
const SW=914.4, SH=609.6, SB=12.7, TB_W=190, TB_H=76;

// Model bounds
let mxMin=Infinity,myMin=Infinity,mxMax=-Infinity,myMax=-Infinity;
DATA.rooms.forEach(r=>r.vertices.forEach(v=>{{mxMin=Math.min(mxMin,v.x);myMin=Math.min(myMin,v.y);mxMax=Math.max(mxMax,v.x);myMax=Math.max(myMax,v.y)}}));
DATA.placements.forEach(p=>{{mxMin=Math.min(mxMin,p.x);myMin=Math.min(myMin,p.y);mxMax=Math.max(mxMax,p.x);myMax=Math.max(myMax,p.y)}});
if(mxMin===Infinity){{mxMin=0;myMin=0;mxMax=20;myMax=20}}
mxMin-=1;myMin-=1;mxMax+=1;myMax+=1;

const drawL=SB+15, drawR=SW-SB-15, drawT=SB+15, drawB=SH-SB-TB_H-15;
const drawW=drawR-drawL, drawH=drawB-drawT;
const mW=mxMax-mxMin, mH=myMax-myMin;
const mScale=Math.min(drawW/(mW*1000), drawH/(mH*1000))*1000;
function m2s(mx,my){{return{{x:drawL+(mx-mxMin)*mScale, y:drawB-(my-myMin)*mScale}}}}

function resize(){{W=innerWidth;H=innerHeight;c.width=W*devicePixelRatio;c.height=H*devicePixelRatio;c.style.width=W+'px';c.style.height=H+'px';ctx.setTransform(devicePixelRatio,0,0,devicePixelRatio,0,0)}}
function fit(){{vs=Math.min(W/(SW+40),H/(SH+40));vx=(W-SW*vs)/2;vy=(H-SH*vs)/2}}

let selected=null;

function render(){{
  ctx.fillStyle='#1a1a1e';ctx.fillRect(0,0,W,H);
  ctx.save();ctx.translate(vx,vy);ctx.scale(vs,vs);

  // White sheet
  ctx.fillStyle='#fff';ctx.fillRect(0,0,SW,SH);

  // Borders
  ctx.strokeStyle='#000';ctx.lineWidth=.75;ctx.strokeRect(SB,SB,SW-2*SB,SH-2*SB);
  ctx.lineWidth=.25;ctx.strokeRect(SB+2,SB+2,SW-2*SB-4,SH-2*SB-4);

  // Title block
  const tbx=SW-SB-TB_W, tby=SH-SB-TB_H-2;
  ctx.lineWidth=.5;ctx.strokeRect(tbx,tby,TB_W-2,TB_H);
  ctx.lineWidth=.15;
  [15,30,45,60].forEach(dy=>{{ctx.beginPath();ctx.moveTo(tbx,tby+dy);ctx.lineTo(tbx+TB_W-2,tby+dy);ctx.stroke()}});
  ctx.beginPath();ctx.moveTo(tbx+45,tby);ctx.lineTo(tbx+45,tby+TB_H);ctx.stroke();

  ctx.fillStyle='#888';ctx.font='5px Helvetica,sans-serif';
  ctx.fillText('PROJECT',tbx+3,tby+10);ctx.fillText('PROJECT NO.',tbx+3,tby+25);
  ctx.fillText('SHEET TITLE',tbx+3,tby+40);ctx.fillText('SHEET NO.',tbx+3,tby+55);
  ctx.fillText('FORMAT',tbx+3,tby+70);
  ctx.fillStyle='#000';ctx.font='bold 9px Helvetica,sans-serif';
  ctx.fillText(DATA.project,tbx+48,tby+11);
  ctx.font='7px Helvetica,sans-serif';
  ctx.fillText(DATA.number,tbx+48,tby+26);
  ctx.font='bold 7px Helvetica,sans-serif';
  ctx.fillText(DATA.level,tbx+48,tby+41);
  ctx.font='bold 9px Helvetica,sans-serif';
  ctx.fillText(DATA.sheet,tbx+48,tby+56);
  ctx.fillStyle='#888';ctx.font='5px Helvetica,sans-serif';
  ctx.fillText('SED — Structured Engineering Document',tbx+48,tby+70);

  // Rooms
  DATA.rooms.forEach(r=>{{
    if(r.vertices.length<3)return;
    const sv=r.vertices.map(v=>m2s(v.x,v.y));
    const isSel=selected&&selected.type==='room'&&selected.data.tag===r.tag;
    if(isSel){{ctx.fillStyle='#d0e4ff40';ctx.beginPath();ctx.moveTo(sv[0].x,sv[0].y);for(let i=1;i<sv.length;i++)ctx.lineTo(sv[i].x,sv[i].y);ctx.closePath();ctx.fill()}}
    ctx.strokeStyle=isSel?'#2060c0':(r.scope==='nic'?'#bbb':'#000');
    ctx.lineWidth=isSel?.6:(r.scope==='nic'?.15:.35);
    ctx.beginPath();ctx.moveTo(sv[0].x,sv[0].y);for(let i=1;i<sv.length;i++)ctx.lineTo(sv[i].x,sv[i].y);ctx.closePath();ctx.stroke();
    const cx=sv.reduce((s,v)=>s+v.x,0)/sv.length, cy=sv.reduce((s,v)=>s+v.y,0)/sv.length;
    ctx.fillStyle='#555';ctx.font='bold 5px Helvetica,sans-serif';ctx.textAlign='center';
    ctx.fillText(r.tag,cx,cy-3);
    ctx.fillStyle='#000';ctx.font='4px Helvetica,sans-serif';
    ctx.fillText(r.name,cx,cy+3);
    ctx.textAlign='left';
  }});

  // Duct segments
  DATA.segments.forEach(s=>{{
    const p1=m2s(s.x1,s.y1), p2=m2s(s.x2,s.y2);
    ctx.strokeStyle='#666';ctx.lineWidth=Math.max(s.d*mScale*0.0254*0.8, 0.2);ctx.lineCap='round';
    ctx.beginPath();ctx.moveTo(p1.x,p1.y);ctx.lineTo(p2.x,p2.y);ctx.stroke();
    // Size label at midpoint
    const mx=(p1.x+p2.x)/2, my=(p1.y+p2.y)/2;
    ctx.fillStyle='#888';ctx.font='3px Helvetica,sans-serif';ctx.textAlign='center';
    ctx.fillText(s.d+'"',mx,my-1.5);ctx.textAlign='left';
  }});

  // Nodes
  DATA.nodes.forEach(n=>{{
    const p=m2s(n.x,n.y);
    ctx.fillStyle='#999';ctx.fillRect(p.x-.6,p.y-.6,1.2,1.2);
  }});

  // Placements
  DATA.placements.forEach(p=>{{
    const sp=m2s(p.x,p.y);
    const isSel=selected&&selected.type==='placement'&&selected.data===p;
    const r=p.dom==='equipment'?2.5:1.5;
    ctx.strokeStyle=isSel?'#2060c0':'#000';ctx.lineWidth=isSel?.5:.3;
    if(p.dom==='equipment'){{
      ctx.beginPath();ctx.moveTo(sp.x,sp.y-r);ctx.lineTo(sp.x+r,sp.y);ctx.lineTo(sp.x,sp.y+r);ctx.lineTo(sp.x-r,sp.y);ctx.closePath();ctx.stroke();
    }}else{{
      ctx.beginPath();for(let i=0;i<8;i++){{const a=Math.PI*2*i/8;const ox=sp.x+r*Math.cos(a),oy=sp.y+r*Math.sin(a);i===0?ctx.moveTo(ox,oy):ctx.lineTo(ox,oy)}}ctx.closePath();ctx.stroke();
    }}
    if(isSel){{ctx.strokeStyle='#2060c080';ctx.lineWidth=.3;ctx.setLineDash([.5,.5]);ctx.beginPath();ctx.arc(sp.x,sp.y,r+1.5,0,Math.PI*2);ctx.stroke();ctx.setLineDash([])}}
    ctx.fillStyle='#333';ctx.font='3.5px Helvetica,sans-serif';
    const label=p.cfm?p.tag+' '+p.cfm+'CFM':p.tag;
    ctx.fillText(label,sp.x+r+1,sp.y+1);
  }});

  // Scale bar
  const barM=Math.ceil(5/mScale*1000);const barMM=barM*mScale/1000;
  ctx.strokeStyle='#000';ctx.lineWidth=.5;
  ctx.beginPath();ctx.moveTo(drawL,drawB+8);ctx.lineTo(drawL+barMM,drawB+8);ctx.stroke();
  for(let i=0;i<=barM;i++){{const tx=drawL+i/barM*barMM;ctx.beginPath();ctx.moveTo(tx,drawB+6.5);ctx.lineTo(tx,drawB+9.5);ctx.stroke()}}
  ctx.fillStyle='#000';ctx.font='4px Helvetica,sans-serif';
  ctx.fillText('0',drawL,drawB+13);ctx.fillText(barM+'m',drawL+barMM-5,drawB+13);

  // North arrow
  const nax=drawR-10,nay=drawT+10;
  ctx.fillStyle='#000';ctx.beginPath();ctx.moveTo(nax,nay-8);ctx.lineTo(nax+3,nay);ctx.lineTo(nax,nay-2);ctx.lineTo(nax-3,nay);ctx.closePath();ctx.fill();
  ctx.font='bold 5px Helvetica,sans-serif';ctx.textAlign='center';ctx.fillText('N',nax,nay-10);ctx.textAlign='left';

  ctx.restore();
}}

// Interaction
c.onmousedown=e=>{{dragging=true;dsx=e.clientX-vx;dsy=e.clientY-vy;c.style.cursor='grabbing'}};
c.onmousemove=e=>{{if(dragging){{vx=e.clientX-dsx;vy=e.clientY-dsy;render()}}}};
c.onmouseup=()=>{{dragging=false;c.style.cursor='grab'}};
c.onmouseleave=()=>{{dragging=false;c.style.cursor='grab'}};
c.onwheel=e=>{{e.preventDefault();const f=e.deltaY<0?1.1:.9;const r=c.getBoundingClientRect();const mx=e.clientX-r.left,my=e.clientY-r.top;vx=mx-(mx-vx)*f;vy=my-(my-vy)*f;vs*=f;render()}};

c.ondblclick=e=>{{
  const r=c.getBoundingClientRect();
  const sx=(e.clientX-r.left-vx)/vs, sy=(e.clientY-r.top-vy)/vs;

  // Hit test placements
  for(const p of DATA.placements){{
    const sp=m2s(p.x,p.y);
    const dx=sx-sp.x,dy=sy-sp.y;
    if(Math.sqrt(dx*dx+dy*dy)<4){{
      selected={{type:'placement',data:p}};
      showPlacementProps(p);
      render();return;
    }}
  }}

  // Hit test rooms
  for(const rm of DATA.rooms){{
    if(rm.vertices.length<3)continue;
    const sv=rm.vertices.map(v=>m2s(v.x,v.y));
    if(pointInPoly(sx,sy,sv)){{
      selected={{type:'room',data:rm}};
      showRoomProps(rm);
      render();return;
    }}
  }}

  selected=null;document.getElementById('props').style.display='none';render();
}};

function pointInPoly(px,py,vs){{
  let inside=false;
  for(let i=0,j=vs.length-1;i<vs.length;j=i++){{
    if((vs[i].y>py)!==(vs[j].y>py)&&px<(vs[j].x-vs[i].x)*(py-vs[i].y)/(vs[j].y-vs[i].y)+vs[i].x)inside=!inside;
  }}
  return inside;
}}

function showPlacementProps(p){{
  const panel=document.getElementById('props');
  const devs=DATA.placements.filter(d=>d.room===p.room&&p.room);
  const roomCfm=devs.reduce((s,d)=>s+(d.cfm||0),0);
  let h='<h3>'+esc(p.itag||p.tag)+'<span class="close" onclick="closeProps()">&times;</span></h3>';
  h+=pr('Tag',p.tag);h+=pr('Category',p.cat);h+=pr('Domain',p.dom);
  h+=pr('Manufacturer',p.mfr||'-');h+=pr('Model',p.model||'-');
  h+=pr('CFM',p.cfm||'-');h+=pr('Room',p.room||'-');
  h+=pr('Position',p.x.toFixed(2)+', '+p.y.toFixed(2)+' m');
  if(p.room&&roomCfm)h+=pr('Room Total CFM',roomCfm);
  panel.innerHTML=h;panel.style.display='block';
}}

function showRoomProps(r){{
  const panel=document.getElementById('props');
  const devs=DATA.placements.filter(p=>{{
    for(const v of r.vertices){{const sp=m2s(v.x,v.y)}}
    return false;
  }});
  // Find placements in this room by checking room name
  const roomDevs=DATA.placements.filter(p=>p.room===r.name);
  const totalCfm=roomDevs.reduce((s,d)=>s+(d.cfm||0),0);
  let h='<h3>'+esc(r.tag)+' — '+esc(r.name)+'<span class="close" onclick="closeProps()">&times;</span></h3>';
  h+=pr('Scope',r.scope);h+=pr('Devices',roomDevs.length);h+=pr('Total CFM',totalCfm||'-');
  if(roomDevs.length){{
    h+='<div style="margin-top:8px;padding-top:6px;border-top:1px solid #ddd;font-size:11px;color:#666;text-transform:uppercase;letter-spacing:.5px">Devices</div>';
    roomDevs.forEach(d=>{{h+=pr(d.tag,(d.cfm||'-')+' CFM')}});
  }}
  panel.innerHTML=h;panel.style.display='block';
}}

function pr(k,v){{return '<div class="pr"><span class="k">'+k+'</span><span class="v">'+v+'</span></div>'}}
function esc(s){{return s?String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'):''}}
function closeProps(){{selected=null;document.getElementById('props').style.display='none';render()}}
document.onkeydown=e=>{{if(e.key==='Escape')closeProps()}};

resize();fit();render();
onresize=()=>{{resize();render()}};
</script></body></html>"##,
        project = esc_html(&info.project_name),
        number = esc_html(&info.project_number),
        level = esc_html(level),
        sheet_num = "M-101",
        rooms = rooms_json.join(","),
        placements = place_json.join(","),
        segments = seg_json.join(","),
        nodes = node_json.join(","),
    );

    std::fs::write(output, html)?;
    println!("Exported: {}", output);
    Ok(())
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")
}

fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
