### 介绍

NexTrade 开源商城系统是一款全开源可商用的系统，前后端分离开发，拥抱最新的 PHP8+ VUE3 uniapp 全新技术架构。

文档祥见：https://github.com/lanline123/NexTrade-Postgres

前端运行 1.下载并且安装 nodejs (版本要求 19.x 以上版本)

# 进入项目目录

cd pc

# 安装依赖

npm install

## 强烈建议不要用直接使用 cnpm 安装，会有各种诡异的 bug，可以通过重新指定 registry 来解决 npm 安装速度慢的问题。

npm install --registry=https://registry.npmmirror.com

2.打开项目根目录下的 .env.development ，基础域名（ VITE_API_URL ）修改成自己的，或者直接使用官方域名（ https://battery.recy.cc ）

# 本地开发 启动项目

npm run dev

3.打开浏览器, 输入( http://localhost:3000/ )

默认账户/密码( 123123/123123 )

若能正确展示商城界面，菜单及页面展示正常，则表明环境搭建成功
前端部署

1.  .env.production 内的基础域名（ VITE_API_URL ）替换成，您自己的域名。
2.  当项目开发完毕，只需要运行一行命令就可以打包你的应用 npm run generate
3.  构建打包成功之后，会在根目录生成 .output 文件夹，里面就是构建打包好的文件。
4.  通常情况下 .output 文件夹的静态文件发布到你的 nginx 或者静态服务器即可。

环境变量
所有测试环境或者正式环境变量的配置都在 .env.development 等 .env.xxxx 文件中。
它们都会注入到全局。你在代码中可以通过如下方式获取:
console.log(process.env.VITE_XXX)
