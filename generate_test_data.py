from flask_sqlalchemy import SQLAlchemy
from datetime import datetime, timedelta
import os
import random
from flask import Flask
import argparse

# 创建Flask应用（仅用于数据库操作）
app = Flask(__name__)
basedir = os.path.abspath(os.path.dirname(__file__))
app.config['SQLALCHEMY_DATABASE_URI'] = f'sqlite:///{os.path.join(basedir, "time_tracker.db")}'
app.config['SQLALCHEMY_TRACK_MODIFICATIONS'] = False
db = SQLAlchemy(app)

# 时间记录模型
class TimeEntry(db.Model):
    id = db.Column(db.Integer, primary_key=True)
    start_time = db.Column(db.DateTime, nullable=False)
    end_time = db.Column(db.DateTime, nullable=False)
    activity = db.Column(db.String(200), nullable=False)
    category = db.Column(db.String(50), nullable=False)
    description = db.Column(db.Text)

def generate_test_data(days=30):
    # 活动名称和对应的类别
    activities = {
        '工作': ['编写代码', '开会', '项目规划', '代码审查', '修复bug', '系统维护'],
        '学习': ['阅读技术书籍', '在线课程学习', '编程练习', '算法学习', '英语学习'],
        '运动': ['跑步', '健身', '游泳', '瑜伽', '篮球'],
        '休息': ['午休', '小憩', '冥想', '散步'],
        '娱乐': ['看电影', '玩游戏', '听音乐', '阅读小说', '刷视频'],
        '其他': ['购物', '整理房间', '做饭', '社交活动']
    }
    
    # 生成指定天数的数据
    end_date = datetime.now()
    start_date = end_date - timedelta(days=days)
    
    # 清空现有数据
    TimeEntry.query.delete()
    
    # 生成新数据
    current_date = start_date
    total_entries = 0
    
    while current_date <= end_date:
        # 每天生成2-5条记录
        num_entries = random.randint(2, 5)
        last_end_time = current_date.replace(hour=9, minute=0)  # 从早上9点开始
        
        for _ in range(num_entries):
            # 随机选择类别
            category = random.choice(list(activities.keys()))
            # 随机选择该类别下的活动
            activity = random.choice(activities[category])
            
            # 生成开始时间（在上一条记录的结束时间之后）
            start_time = last_end_time + timedelta(minutes=random.randint(30, 120))
            
            # 生成结束时间（活动持续30分钟到3小时）
            duration = timedelta(minutes=random.randint(30, 180))
            end_time = start_time + duration
            
            # 生成描述
            descriptions = [
                f"完成了{activity}的任务",
                f"进行{activity}活动",
                f"专注于{activity}",
                f"处理{activity}相关事项",
                None
            ]
            description = random.choice(descriptions)
            
            # 创建记录
            entry = TimeEntry(
                start_time=start_time,
                end_time=end_time,
                activity=activity,
                category=category,
                description=description
            )
            db.session.add(entry)
            total_entries += 1
            
            # 更新最后结束时间
            last_end_time = end_time
        
        # 移动到下一天
        current_date += timedelta(days=1)
    
    # 提交所有更改
    db.session.commit()
    return total_entries

if __name__ == '__main__':
    # 创建命令行参数解析器
    parser = argparse.ArgumentParser(description='生成时间记录的测试数据')
    parser.add_argument('-d', '--days', type=int, default=30,
                      help='要生成的天数（默认：30天）')
    parser.add_argument('--no-clear', action='store_true',
                      help='不清除现有数据，直接追加新数据')
    
    args = parser.parse_args()
    
    with app.app_context():
        try:
            if not args.no_clear:
                TimeEntry.query.delete()
                print('已清除现有数据')
            
            num_entries = generate_test_data(args.days)
            print(f'成功生成 {num_entries} 条测试数据！')
            print(f'数据范围：{(datetime.now() - timedelta(days=args.days)).strftime("%Y-%m-%d")} 到 {datetime.now().strftime("%Y-%m-%d")}')
        except Exception as e:
            print(f'生成测试数据失败：{str(e)}') 