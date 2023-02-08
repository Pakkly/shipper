
function executeTest(){
    setSceneID(0,'MyApp');
}
function installClick_TESTING(){
    var i =0;
    setSceneID(1);
    setInterval(function(){
        setDownloadProgress(0,i*0.01);
        i++;
    },100)
}
