import { expandGlob } from "https://deno.land/std@0.156.0/fs/mod.ts?s=expandGlob";
import { createSchema } from 'https://esm.sh/genson-js';

let jsons = []

for await (const file of expandGlob(".Takeout/**/*.json")) {
  const text = await Deno.readTextFile(file.path)
  const json = JSON.parse(text)
  jsons.push(json)
}

Deno.writeTextFile("schema.json", JSON.stringify(createSchema(jsons), null, 4))