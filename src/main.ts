import { createApp } from "vue";
import './style.css';
import App from "./App.vue";
import { Window } from '@tauri-apps/api/window';

const appWindow = new Window('main');
document
    .getElementById('titlebar-minimize')
    ?.addEventListener('click', () => appWindow.minimize());
document
    .getElementById('titlebar-maximize')
    ?.addEventListener('click', () => appWindow.toggleMaximize());
document
    .getElementById('titlebar-close')
    ?.addEventListener('click', () => {
        // 这一句的意思是关掉程序，appWindow.destroy()同样会关闭程序
        // 对于有系统托盘（下一篇文章会讲）的程序而言点击关闭按钮并不是想退出程序，而是关闭窗口保留托盘，这时候需要使用appWindow.hide()隐藏主窗口，但是程序并没有被关闭。如果要显示主窗口可以在托盘事件中添加appWindow.show()代码，点击托盘可以再显示主窗口
        appWindow.close()
    });

createApp(App).mount("#app");