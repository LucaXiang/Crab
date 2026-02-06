/**
 * Common Chinese words mapped by their full pinyin (concatenated, no tones).
 * Used to generate prefix entries for multi-syllable candidate matching.
 *
 * When the user types beyond a single syllable (e.g. "nih" after "ni"),
 * the prefix entries enable word candidates like 你好, 你会, etc.
 */
export const pinyinWords: Record<string, string> = {
  // ── Greetings & Common phrases ──
  nihao: '你好', nimen: '你们', nide: '你的', nishi: '你是',
  women: '我们', wode: '我的', woshi: '我是',
  tamen: '他们', tade: '他的', tashi: '他是',
  xiexie: '谢谢', zaijian: '再见', duibuqi: '对不起', meiguanxi: '没关系',
  qingwen: '请问', qingjin: '请进',

  // ── Pronouns & Determiners ──
  zhege: '这个', zhexie: '这些', nage: '那个', naxie: '那些',
  dajia: '大家', ziji: '自己', bieren: '别人', shei: '谁',

  // ── Common verbs ──
  keyi: '可以', buyao: '不要', buyong: '不用', bushi: '不是', buxing: '不行',
  meiyou: '没有', xuyao: '需要', zhidao: '知道', xihuan: '喜欢',
  kaishi: '开始', jieshu: '结束', dengdeng: '等等',
  haode: '好的', shide: '是的', xingde: '行的',

  // ── Numbers & Quantities ──
  yige: '一个', liangge: '两个', sange: '三个', sige: '四个', wuge: '五个',
  liuge: '六个', qige: '七个', bage: '八个', jiuge: '九个', shige: '十个',
  jige: '几个', duoshao: '多少', yixie: '一些', yidian: '一点',
  yifen: '一份', liangfen: '两份', sanfen: '三份',
  yibei: '一杯', liangbei: '两杯', yiwan: '一碗', yiping: '一瓶',
  yizhang: '一张', yiwei: '一位', liangwei: '两位',

  // ── Food: Meat ──
  jirou: '鸡肉', jidan: '鸡蛋', jichi: '鸡翅', jitui: '鸡腿', jitang: '鸡汤',
  niurou: '牛肉', niupai: '牛排', niunan: '牛腩', niunai: '牛奶',
  zhurou: '猪肉', zhupai: '猪排', zhuti: '猪蹄',
  yangrou: '羊肉', yangpai: '羊排', yangtang: '羊汤',
  yupian: '鱼片', yutou: '鱼头', yuwan: '鱼丸',
  xiaren: '虾仁', xiajiao: '虾饺', longxia: '龙虾',
  haixian: '海鲜', haidai: '海带',

  // ── Food: Rice, Noodles, Dumplings ──
  mifan: '米饭', mifen: '米粉', mixian: '米线',
  miantiao: '面条', mianbao: '面包',
  chaofan: '炒饭', chaomian: '炒面', chaocai: '炒菜', chaofen: '炒粉',
  jiaozi: '饺子', baozi: '包子', mantou: '馒头', huntun: '馄饨',
  lamian: '拉面', tangmian: '汤面', tangbao: '汤包', zhumian: '煮面',
  jianbing: '煎饼', juanbing: '卷饼', jiandan: '煎蛋',
  gaifan: '盖饭', gaijiaofan: '盖浇饭',
  xiaolongbao: '小笼包',

  // ── Food: Vegetables ──
  qingcai: '青菜', baicai: '白菜', bocai: '菠菜',
  tudou: '土豆', tudousi: '土豆丝', fanqie: '番茄',
  qingjiao: '青椒', huanggua: '黄瓜',
  doufu: '豆腐', douya: '豆芽', doujiang: '豆浆',
  mogu: '蘑菇', xianggu: '香菇', xiangcai: '香菜', xiangchang: '香肠',
  yumi: '玉米', nangua: '南瓜', shanyao: '山药',
  xilanhua: '西兰花', xihongshi: '西红柿',
  huasheng: '花生', huajiao: '花椒', huajuan: '花卷',
  conghua: '葱花', suanrong: '蒜蓉', zicai: '紫菜',

  // ── Food: Cooking styles ──
  hongshao: '红烧', qingzheng: '清蒸', qingchao: '清炒', qingtang: '清汤',
  ganguo: '干锅', huoguo: '火锅', tieban: '铁板',
  shaokao: '烧烤', kaorou: '烤肉', kaoyu: '烤鱼', kaoya: '烤鸭', kaochuan: '烤串',
  tangcu: '糖醋', mala: '麻辣', suanla: '酸辣',
  suancai: '酸菜', liangban: '凉拌', liangcai: '凉菜',
  yuxiangrousi: '鱼香肉丝', huiguorou: '回锅肉', mapodoufu: '麻婆豆腐',

  // ── Drinks ──
  kafei: '咖啡', kale: '咖喱', kele: '可乐',
  pijiu: '啤酒', guozhi: '果汁',
  naicha: '奶茶', naiyou: '奶油', suannai: '酸奶',
  hongjiu: '红酒', baijiu: '白酒',
  hongcha: '红茶', lvcha: '绿茶', mocha: '抹茶',
  reshui: '热水', bingshui: '冰水', bingkuai: '冰块',
  yinliao: '饮料', yinpin: '饮品', xuebi: '雪碧',

  // ── Condiments ──
  jiangyou: '酱油', jiangliao: '酱料',
  lajiao: '辣椒', lajiang: '辣酱',
  hujiao: '胡椒', zhima: '芝麻', zhishi: '芝士',
  baitang: '白糖', hongtang: '红糖', shiyan: '食盐',

  // ── Desserts & Snacks ──
  dangao: '蛋糕', danta: '蛋挞', binggan: '饼干',
  tianpin: '甜品', tiandian: '甜点',
  shuiguo: '水果', pingguo: '苹果', xigua: '西瓜', putao: '葡萄',
  juzi: '橘子', ningmeng: '柠檬', taozi: '桃子',

  // ── Restaurant operations ──
  waimai: '外卖', waidai: '外带', tangshi: '堂食',
  dabao: '打包', dazhe: '打折',
  jiala: '加辣', jiabing: '加冰', jiatang: '加糖', jialiang: '加量', jiada: '加大',
  bula: '不辣', weila: '微辣', zhongla: '中辣', tela: '特辣',
  dafen: '大份', zhongfen: '中份', xiaofen: '小份',
  dawan: '大碗', xiaowan: '小碗',
  dabei: '大杯', zhongbei: '中杯', xiaobei: '小杯',
  daping: '大瓶', xiaoping: '小瓶',
  shaofang: '少放', shaola: '少辣', shaoyan: '少盐', shaotang: '少糖',
  duojia: '多加', duogei: '多给',
  changwen: '常温', zhengchang: '正常',
  beizhu: '备注', tebie: '特别', tese: '特色', tejia: '特价',

  // ── Menu categories ──
  recai: '热菜', lengcai: '冷菜', zhushi: '主食',
  reyin: '热饮', lengyin: '冷饮', xiaochi: '小吃',
  taocan: '套餐', pinpan: '拼盘', zhoupin: '粥品',
  caidan: '菜单', caipin: '菜品',

  // ── Business & Payment ──
  jiezhang: '结账', jiesuan: '结算',
  zhaoling: '找零', zhaoqian: '找钱',
  fukuan: '付款', zhifu: '支付', xianjin: '现金',
  zhuanzhang: '转账', weixin: '微信',
  kaipiao: '开票', kaitai: '开台', kaimen: '开门',
  zhuohao: '桌号', huanzhuo: '换桌',
  zhangdan: '账单', danjia: '单价', dandian: '单点',
  zonge: '总额', zongji: '总计',
  youhui: '优惠', manjian: '满减', zhekou: '折扣',
  shangpin: '商品', jiage: '价格', shuliang: '数量', fenliang: '份量',
  renshu: '人数', keren: '客人', kehu: '客户',
  fuwu: '服务', fuwuyuan: '服务员',
  yuding: '预订', dianhua: '电话', dizhi: '地址',
  laoban: '老板', yuangong: '员工',
  shouju: '收据', shouyin: '收银',
  chufang: '厨房', chucai: '出餐',
  quxiao: '取消', qucan: '取餐', songcan: '送餐',
  tuikuan: '退款', tuicai: '退菜', wancheng: '完成',
  shezhi: '设置', kuaican: '快餐',
  kuaizi: '筷子', panzi: '盘子', beizi: '杯子', wanzi: '碗', shaozi: '勺子',

  // ── Time ──
  jintian: '今天', mingtian: '明天', zuotian: '昨天',
  zaocan: '早餐', wucan: '午餐', wancan: '晚餐', yexiao: '夜宵',
  zaoshang: '早上', zhongwu: '中午', xiawu: '下午', wanshang: '晚上',
  shijian: '时间', fenzhong: '分钟', xiaoshi: '小时',

  // ── Common 2-char words ──
  yinwei: '因为', suoyi: '所以', danshi: '但是', ruguo: '如果',
  haishi: '还是', yijing: '已经', zhiyao: '只要', zhiyou: '只有',
  ranhou: '然后', xianzai: '现在', yihou: '以后', yiqian: '以前',
  zuihou: '最后', zuijin: '最近', yiqi: '一起', yiyang: '一样',
  feichang: '非常', keneng: '可能', yinggai: '应该',
  wenhua: '文化', dongxi: '东西', difang: '地方', wenti: '问题',
  banfa: '办法', pengyou: '朋友', gongzuo: '工作',
  diannao: '电脑', shouji: '手机', yinhang: '银行',
};
