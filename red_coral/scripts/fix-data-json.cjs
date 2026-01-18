const fs = require('fs');
const path = require('path');

const filePath = path.join(__dirname, '../data.json');

try {
  if (!fs.existsSync(filePath)) {
    console.error('data.json not found at:', filePath);
    process.exit(1);
  }

  const rawData = fs.readFileSync(filePath, 'utf8');
  const data = JSON.parse(rawData);
  let modified = false;

  // Fix attribute_templates
  if (data.attribute_templates && Array.isArray(data.attribute_templates)) {
    data.attribute_templates.forEach(template => {
      if (template.receiptName === undefined) {
        template.receiptName = null;
        modified = true;
      }
      if (template.kitchenPrinterId === undefined) {
        template.kitchenPrinterId = null;
        modified = true;
      }
    });
    console.log(`Processed ${data.attribute_templates.length} attribute templates.`);
  }

  // Fix attribute_options
  if (data.attribute_options && Array.isArray(data.attribute_options)) {
    data.attribute_options.forEach(option => {
      if (option.receiptName === undefined) {
        option.receiptName = null;
        modified = true;
      }
    });
    console.log(`Processed ${data.attribute_options.length} attribute options.`);
  }

  if (modified) {
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf8');
    console.log('Successfully updated data.json with missing fields.');
  } else {
    console.log('No changes needed for data.json.');
  }

} catch (error) {
  console.error('Error processing data.json:', error);
}
