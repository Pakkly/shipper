const fs = require('fs');
const path = require('path');
if(!fs.existsSync(path.resolve(__dirname,"node_modules"))){
  //run npm i
  let currentDir = process.cwd();
  process.chdir(__dirname);
  const exec = require('child_process').execSync;
  try{
    exec('npm i');
    process.chdir(currentDir);
  }catch(e){
    process.chdir(currentDir);
    throw e;
  }
}

let os;
if(process.platform === 'win32'){
    os = 'windows'
}
if(process.platform === 'linux'){
    os = 'linux'
}
if(process.platform === 'darwin'){
    os = 'macos'
}
const { inlineSource } = require('inline-source');
const yargs = require('yargs/yargs')
const { hideBin } = require('yargs/helpers')
const argv = yargs(hideBin(process.argv)).argv
const htmlpath = path.resolve(argv.entrypoint);
var minify = require('html-minifier').minify;
 
inlineSource(htmlpath, {
  compress: true,
  rootpath: argv.root,
  attribute: false,
  svgAsImage: true
})
  .then((html) => {
    let replaced = html.replace('TARGET_OS_REPLACE',os);
      var result = minify(replaced, {
        collapseWhitespace: true,
        removeComments: true,
        removeOptionalTags:true,
        removeScriptTypeAttributes:true,
        removeTagWhitespace:true,
        removeAttributeQuotes:true,
        useShortDoctype:true,
        removeRedundantAttributes:true,
        minifyCSS:true,
        minifyJS:true,
      });
      console.log(result)
  })
  .catch((err) => {
    console.log(JSON.stringify(err));
    process.exit(1);
  });