const localePath = "../src/services/i18n/locales/zh-CN.json";
const content = await Deno.readTextFile(localePath);
const data = JSON.parse(content);

// 添加缺失的 keys
data.settings.common = {
  ...(data.settings.common || {}),
  untitled: "未命名",
  properties: "属性",
  actions: "操作"
};
data.settings.printer.kitchenPrinting = {
  ...(data.settings.printer.kitchenPrinting || {}),
  disabled: "厨房打印已关闭",
  enableToConfigure: "启用厨房打印后，您可以配置多个工位、路由规则和小票格式"
};

await Deno.writeTextFile(localePath, JSON.stringify(data, null, 2) + '\n');
console.log("✅ 已添加 5 个缺失的 keys");
