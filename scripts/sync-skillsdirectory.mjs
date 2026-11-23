#!/usr/bin/env node
// Metadata sync only. Requires SKILLSDIRECTORY_API_KEY. Does not mirror content.
const base="https://www.skillsdirectory.com/api/v1";
const key=process.env.SKILLSDIRECTORY_API_KEY;
if(!key){ console.error("Missing SKILLSDIRECTORY_API_KEY"); process.exit(2); }
const limit=Math.min(100,Math.max(1,Number(process.env.PAGE_SIZE||100)));
const maxPages=Math.max(1,Number(process.env.MAX_PAGES||1));
const rows=[];
for(let page=0;page<maxPages;page++){
  const url=new URL(base+"/skills");
  url.searchParams.set("limit",String(limit));
  url.searchParams.set("offset",String(page*limit));
  url.searchParams.set("securityGrade",process.env.SECURITY_GRADE||"A");
  const res=await fetch(url,{headers:{Authorization:"Bearer "+key}});
  if(!res.ok) throw new Error("SkillsDirectory API "+res.status+": "+await res.text());
  const json=await res.json(); rows.push(...(json.data||[]));
  if(!json.pagination?.hasNextPage) break;
}
process.stdout.write(JSON.stringify({registry:base,fetchedAt:new Date().toISOString(),count:rows.length,skills:rows.map(s=>({slug:s.slug,name:s.name,description:s.description,author:s.author,source:s.githubUrl||s.sourceUrl||null,license:s.license||null,securityGrade:s.securityGrade||null,securityScore:s.securityScore??null}))},null,2)+"\n");
