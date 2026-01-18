const localePath = "../src/services/i18n/locales/zh-CN.json";
const content = await Deno.readTextFile(localePath);
const data = JSON.parse(content);

// 添加缺失的 keys - statistics.overview 是字符串，需要用不同的路径
if (!data.statistics.metric) data.statistics.metric = {};
data.statistics.metric.orders = "订单";

if (!data.checkout.timeline) data.checkout.timeline = {};
data.checkout.timeline.label = "标签";
data.checkout.timeline.title = "操作记录";

// pos.quickAdd 可能已存在，需要合并
if (!data.pos.quickAdd) {
  data.pos.quickAdd = {};
}
data.pos.quickAdd.title = "快速添加";
data.pos.quickAdd.noProducts = "暂无可用商品";
data.pos.quickAdd.selectedItems = "已选项目";
data.pos.quickAdd.selectPrompt = "请从左侧选择商品";
data.pos.quickAdd.perUnit = "/份";
data.pos.quickAdd.confirm = "确认添加";

await Deno.writeTextFile(localePath, JSON.stringify(data, null, 2) + '\n');
console.log("✅ 已添加 9 个缺失的 keys");
