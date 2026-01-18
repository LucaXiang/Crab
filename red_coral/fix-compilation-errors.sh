#!/bin/bash
# 批量修复编译错误的脚本

echo "开始批量修复编译错误..."

# 1. 修复 displayName -> display_name
echo "修复 displayName -> display_name..."
find src/ -name "*.tsx" -o -name "*.ts" | xargs sed -i '' 's/\.displayName/\.display_name/g'

# 2. 修复 role -> role_id (针对 User 类型)
echo "修复 role -> role_id (User 类型)..."
find src/ -name "*.tsx" -o -name "*.ts" | xargs sed -i '' 's/editingUser\.role/editingUser.role_id/g'

# 3. 修复 isActive -> is_active
echo "修复 isActive -> is_active..."
find src/ -name "*.tsx" -o -name "*.ts" | xargs sed -i '' 's/\.isActive/\.is_active/g'

# 4. 修复 zoneId -> zone_id
echo "修复 zoneId -> zone_id..."
find src/ -name "*.tsx" -o -name "*.ts" | xargs sed -i '' 's/\.zoneId/\.zone_id/g'

# 5. 修复 tableId -> table_id
echo "修复 tableId -> table_id..."
find src/ -name "*.tsx" -o -name "*.ts" | xargs sed -i '' 's/\.tableId/\.table_id/g'

# 6. 修复 categoryId -> category_id
echo "修复 categoryId -> category_id..."
find src/ -name "*.tsx" -o -name "*.ts" | xargs sed -i '' 's/\.categoryId/\.category_id/g'

# 7. 修复 productId -> product_id
echo "修复 productId -> product_id..."
find src/ -name "*.tsx" -o -name "*.ts" | xargs sed -i '' 's/\.productId/\.product_id/g'

# 8. 修复 attributeId -> attribute_id
echo "修复 attributeId -> attribute_id..."
find src/ -name "*.tsx" -o -name "*.ts" | xargs sed -i '' 's/\.attributeId/\.attribute_id/g'

# 9. 修复 optionId -> option_id
echo "修复 optionId -> option_id..."
find src/ -name "*.tsx" -o -name "*.ts" | xargs sed -i '' 's/\.optionId/\.option_id/g'

# 10. 修复 specificationId -> specification_id
echo "修复 specificationId -> specification_id..."
find src/ -name "*.tsx" -o -name "*.ts" | xargs sed -i '' 's/\.specificationId/\.specification_id/g'

echo "批量修复完成！"
echo "请重新运行编译检查：npx tsc --noEmit --skipLibCheck"
