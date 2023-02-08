var buildtime_target_os = "TARGET_OS_REPLACE";// will be filled with target os by build_tools/inline_source
function locateID(id){
    return document.getElementById(id);
}

function getImageID(image){
    switch(image){
        case 0: return 'NoInternet'
        case 1: return 'Error'
        case 2: return 'Question'
    }
}




/*
window.addEventListener('contextmenu', function (e) { 
    e.preventDefault(); 
  }, false);
*/

function stopDrag(e){
    e = e || window.event
    e.preventDefault();
    console.log("Drag stop")
    document.onmouseup = null;
    document.onmousemove = null;
}
var initialPos;
function dragged(e){
    e = e || window.event
    e.preventDefault();
    var newPos = {x:e.clientX,y:e.clientY};
    var diff = {x: (newPos.x-initialPos.x), y: (newPos.y-initialPos.y)}
    console.log(diff);

}
function startDrag(e){
    e = e || window.event
    e.preventDefault();
    console.log("Drag start")
    initialPos = {x:e.clientX,y:e.clientY};
    document.onmouseup = stopDrag
    document.onmousemove = dragged;
}
locateID('dragHandle').onmousedown = startDrag;


function showAlert(title, body, image, yesString, noString){
    var titleElem = locateID('title')
    var descriptionElem = locateID('description')
    var yesElem = locateID('yes')
    var noElem = locateID('no')
    if(noString.length == 0){
        
        //workaround for IE7
        noElem.parentNode.removeChild(noElem);
    }

    titleElem.innerText = title;
    descriptionElem.innerText = body

    var correctImage = locateID(getImageID(image));
    correctImage.className += " visible"
    yesElem.innerText = yesString;
    noElem.innerText = noString;
}
if(buildtime_target_os === 'linux'){
    window.external={invoke:function(x){{window.webkit.messageHandlers.external.postMessage(x);}}};
}
external.invoke("init")