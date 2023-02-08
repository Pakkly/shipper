var buildtime_target_os = "TARGET_OS_REPLACE";// will be filled with target os by build_tools/inline_source
document.ondragstart = function () { return false; };
var interval = 10000;
var sektor2 = new Sektor('#pBar', {
    size: 200,
    stroke: 20,
    arc: true,
    angle: 359.9,
    sectorColor: '#7EEB58',
    circleColor: "#2f2f2f",
    fillCircle: false,
    additionalSVG: '<circle\n      id=\'anim-circle\' stroke-width=0 fill=#FF0000 stroke=none      cx=\'100\'      cy=\'100\' r=\'0\' />'
});
var sektorSvg = document.getElementsByClassName("Sektor")[0];
var pBar = document.getElementById('pBar')


//var circleSvg = '<circle\n      id=\'anim-circle\' stroke-width=0 fill=#FF0000 stroke=none      cx=\'100\'      cy=\'100\' r=\'0\' ><animate attributeName="r" begin="0s" dur="1s" repeatCount="indefinite" from="0" to="80"></circle>';

var pText = document.getElementById('pText');
var iText = document.getElementById('headerText');

var currentScene;//Scenes= {0: "Click to Install", 1: "Install Progress"}

var animCircleElem = document.getElementById('anim-circle');
var expandStart,expandDuration=1000,expandFinal=82;



function expandToInstall(timestamp){
    if (expandStart === undefined){
        expandStart = timestamp;
    }
    var elapsed = timestamp - expandStart;

    var ratio = elapsed / expandDuration
    animCircleElem.setAttribute('r',(ratio*expandFinal).toString())
    if(ratio > 1){
        currentScene = 1;
        setDownloadProgress(0,0);
        removeClass(pText,'fadeout');
        return;
    }else{
        requestAnimationFrame(expandToInstall);
    }

}
function setDownloadProgress(segment,progRatio){
    if(currentScene === 0 || currentScene === undefined) return;//ignore events until installer ready.

    var downloading = segment === 0;
    var trueRatio = progRatio*0.5 + (downloading?0:0.5);
    var angle = trueRatio * 360;
    sektor2.animateTo(angle,1000);
    setIfNotIdentical(pText,'innerText',((trueRatio*100).toFixed(0))+"%")
    setIfNotIdentical(iText,'innerText',segment === 0 ? "Downloading..." : "Installing...")
}
function setSceneID(sceneID,appName){
    if(sceneID === 0){
        //pText.innerText = "Install";
        pText.innerText = "";
        iText.innerText = appName
        var clickableElems = [
            document.getElementsByClassName('Sektor-circle')[0],
            document.getElementsByClassName('Sektor-sector')[0],
            pText
        ]
        var clickableClass='clickable';
        var clicked = false;

        var f = function(event){
            if(clicked) return;
            clicked = true;
            for(var i=0;i<clickableElems.length;i++){
                removeClass(clickableElems[i],clickableClass)
            }
            
            if(BROWSER) installClick_TESTING()
            else external.invoke("download")
        }
        setTimeout(f,500);
        for(var i=0;i<clickableElems.length;i++){
            //clickableElems[i].addEventListener('click',f);
            addClass(clickableElems[i],clickableClass)
        }
        currentScene = sceneID;
    }
    if(sceneID === 1){
        //Start playing the transition animation.
        addClass(pText,'fadeout');
        requestAnimationFrame(expandToInstall);
        return;
    }
}
if(BROWSER){
    executeTest()
}else{
    if(buildtime_target_os === 'linux'){
        window.external={invoke:function(x){{window.webkit.messageHandlers.external.postMessage(x);}}};
    }
    external.invoke("init")
}