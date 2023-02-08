
function setIfNotIdentical(obj,propName,propValue){
    if(obj[propName] !== propValue)    
        obj[propName] = propValue
}
function addClass(elem,className){
    var classNames = elem.getAttribute('class');
    if(!classNames) {
        classNames = ''
    }
    classNames = classNames.split(' ')
    classNames.push(className);
    elem.setAttribute('class',classNames.join(' '));
}
function removeClass(elem,className){
    var classNames = elem.getAttribute('class').split(' ');
    var cnPure = []
    for(var j =0;j<classNames.length;j++){
        //i am sorry, little one. Needs to be IE7 compatible.
        if(classNames[j] !== className){
            cnPure.push(classNames[j]);
        }
    }
    elem.setAttribute('class',cnPure.join(' '));
}