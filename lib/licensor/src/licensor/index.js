
const fs = require('fs')


const path = require('path')
const exec = require('child_process').execSync
const licenseLib = require('./licenses')
const licenseWhitelist = [
    'shipper',
    'compile_time',
    'html_embed',
    'pakkly_error',
    'licensor',
    'windows_interface'
]
function compileLicensors(lic){
    if(lic.name === 'UNKNOWN LICENSE') return '';
    let licenseFileString = ''
    licenseFileString += `3rd-party libraries licensed under the ${lic.name}:\n`
    lic.list.map(x=>{
        licenseFileString += `'${x.name}' by ${x.authors} \n`
    });
    if(lic.text.length > 0){
        licenseFileString += `${lic.name}: \n`
        licenseFileString += lic.text;
    }
    return licenseFileString
}

let root = process.cwd();
while(true){
    let filelist = fs.readdirSync(root);
    if(filelist.indexOf('Cargo.toml') > -1){
        break;
    }
    root = path.resolve(root,'..')
}
console.log(`root: ${root}`);
const licenses = Array.from(JSON.parse(exec('cargo license -j').toString('utf-8')))

const licObj = {
    MIT:{name:'MIT License',text:licenseLib.MIT,list:[]},
    Apache:{name:'Apache License, Version 2.0',text:licenseLib.APACHE2,list:[]},
    MPL2:{name:'Mozilla Public License Version 2.0',text:licenseLib.MPL2,list:[]},
    BSD2Clause:{name:'2-Clause BSD License',text:licenseLib.BSD2CLAUSE,list:[]},
    ISC:{name:'ISC License',text:licenseLib.ISC,list:[]},
    ZLIBACK:{name:'zlib License with Acknowledgement',text:licenseLib.ZLIBACK,list:[]},
    PUBLICDOMAIN:{name:'Public Domain',text:'',list:[]},
    ring:{name:'ring License',text:licenseLib.ring,list:[]},
    webpki:{name:'webpki License',text:licenseLib.webpki,list:[]},
    Other:{name:'UNKNOWN LICENSE',text:'',list:[]}
}
for(const lib of licenses){
    if(lib.name == 'ring'){
        licObj.ring.list.push(lib);
        continue;
    }
    if(lib.name == 'webpki'){
        licObj.ring.list.push(lib);
        continue;
    }
    if(licenseWhitelist.indexOf(lib.name) > -1){
        continue;//unlicensed.
    }
    if(!lib.license){
        licObj.Other.list.push(lib);
        continue;
    }
    let regex = lib.license.match(/(OR|^)\s*(CC0-1.0)(?=\s*($|OR))/igm)
    if(regex){
        licObj.PUBLICDOMAIN.list.push(lib);
        continue;
    }
    
    
    regex = lib.license.match(/(OR|^)\s*(MIT)(?=\s*($|OR))/igm)
    if(regex){
        licObj.MIT.list.push(lib);
        continue;
    }
    
    
    regex = lib.license.match(/(OR|^)\s*(Apache-2\.0)(?=\s*($|OR))/igm)
    if(regex){
        licObj.Apache.list.push(lib);
        continue;
    }
    
    
    regex = lib.license.match(/(OR|^)\s*(BSD-2-Clause)(?=\s*($|OR))/igm)
    if(regex){
        licObj.BSD2Clause.list.push(lib);
        continue;
    }


    regex = lib.license.match(/(OR|^)\s*(ISC)(?=\s*($|OR))/igm)
    if(regex){
        licObj.ISC.list.push(lib);
        continue;
    }


    regex = lib.license.match(/(OR|^)\s*(zlib-acknowledgement)(?=\s*($|OR))/igm)
    if(regex){
        licObj.ZLIBACK.list.push(lib);
        continue;
    }


    regex = lib.license.match(/(OR|^)\s*(MPL-2.0)(?=\s*($|OR))/igm)
    if(regex){
        licObj.MPL2.list.push(lib);
        continue;
    }


    licObj.Other.list.push(lib);
}
if(licObj.Other.list.length > 0){
    console.log(`Libs missing licenses: ${licObj.Other.list.length}`)
    console.log(`Libs missing licenses: ${licObj.Other.list.map(x=>x.name+":"+x.license).join(',')}`)
    process.exit(1);
}
let licenseFileString = ''
for(lic of Object.keys(licObj)){
    licenseFileString += compileLicensors(licObj[lic])+'\n\n\n\n\n';
}



console.log(`out: ${process.argv[2]}`);
fs.writeFileSync(process.argv[2],licenseFileString);