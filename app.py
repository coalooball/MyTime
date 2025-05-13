from flask import Flask, render_template, request, redirect, url_for, flash, jsonify
from flask_sqlalchemy import SQLAlchemy
from datetime import datetime, timedelta
import os

app = Flask(__name__)
app.config['SECRET_KEY'] = os.urandom(24)

# 获取项目根目录的绝对路径
basedir = os.path.abspath(os.path.dirname(__file__))
# 设置数据库文件路径为项目根目录下的 time_tracker.db
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

# 存储当前正在进行的活动
current_activity = {
    'start_time': None,
    'activity': None,
    'category': None,
    'description': None
}

@app.route('/')
def index():
    return render_template('index.html')

@app.route('/time_entry', methods=['GET', 'POST'])
def time_entry():
    if request.method == 'POST':
        try:
            start_time = datetime.strptime(request.form['start_time'], '%Y-%m-%dT%H:%M')
            end_time = datetime.strptime(request.form['end_time'], '%Y-%m-%dT%H:%M')
            
            if end_time <= start_time:
                flash('结束时间必须晚于开始时间', 'danger')
                return redirect(url_for('time_entry'))
            
            entry = TimeEntry(
                start_time=start_time,
                end_time=end_time,
                activity=request.form['activity'],
                category=request.form['category'],
                description=request.form['description']
            )
            db.session.add(entry)
            db.session.commit()
            flash('时间记录已保存', 'success')
            return redirect(url_for('time_entry'))
        except Exception as e:
            flash(f'保存失败，请检查输入: {str(e)}', 'danger')
            return redirect(url_for('time_entry'))
    
    # 获取今日记录
    today = datetime.now().date()
    today_entries = TimeEntry.query.filter(
        db.func.date(TimeEntry.start_time) == today
    ).order_by(TimeEntry.start_time.desc()).all()
    
    # 获取当前时间作为默认值
    now = datetime.now()
    next_hour = now + timedelta(hours=1)
    
    return render_template('time_entry.html', 
                         entries=today_entries, 
                         now=now,
                         next_hour=next_hour,
                         current_activity=current_activity)

@app.route('/start_activity', methods=['POST'])
def start_activity():
    global current_activity
    try:
        data = request.get_json()
        current_activity = {
            'start_time': datetime.now(),
            'activity': data['activity'],
            'category': data['category'],
            'description': data.get('description', '')
        }
        return jsonify({'status': 'success', 'message': '活动已开始'})
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 400

@app.route('/end_activity', methods=['POST'])
def end_activity():
    global current_activity
    try:
        if not current_activity['start_time']:
            return jsonify({'status': 'error', 'message': '没有正在进行的活动'}), 400

        entry = TimeEntry(
            start_time=current_activity['start_time'],
            end_time=datetime.now(),
            activity=current_activity['activity'],
            category=current_activity['category'],
            description=current_activity['description']
        )
        db.session.add(entry)
        db.session.commit()

        # 重置当前活动
        current_activity = {
            'start_time': None,
            'activity': None,
            'category': None,
            'description': None
        }

        return jsonify({'status': 'success', 'message': '活动已结束'})
    except Exception as e:
        return jsonify({'status': 'error', 'message': str(e)}), 400

@app.route('/get_current_activity')
def get_current_activity():
    if current_activity['start_time']:
        duration = datetime.now() - current_activity['start_time']
        return jsonify({
            'status': 'success',
            'activity': current_activity['activity'],
            'category': current_activity['category'],
            'start_time': current_activity['start_time'].strftime('%Y-%m-%d %H:%M:%S'),
            'duration': str(duration).split('.')[0]  # 移除微秒
        })
    return jsonify({'status': 'success', 'activity': None})

@app.route('/edit_entry/<int:entry_id>', methods=['GET', 'POST'])
def edit_entry(entry_id):
    entry = TimeEntry.query.get_or_404(entry_id)
    
    if request.method == 'POST':
        try:
            start_time = datetime.strptime(request.form['start_time'], '%Y-%m-%dT%H:%M')
            end_time = datetime.strptime(request.form['end_time'], '%Y-%m-%dT%H:%M')
            
            if end_time <= start_time:
                flash('结束时间必须晚于开始时间', 'danger')
                return redirect(url_for('edit_entry', entry_id=entry_id))
            
            entry.start_time = start_time
            entry.end_time = end_time
            entry.activity = request.form['activity']
            entry.category = request.form['category']
            entry.description = request.form['description']
            
            db.session.commit()
            flash('时间记录已更新', 'success')
            return redirect(url_for('time_entry'))
        except Exception as e:
            flash(f'更新失败，请检查输入: {str(e)}', 'danger')
            return redirect(url_for('edit_entry', entry_id=entry_id))
    
    return render_template('edit_entry.html', entry=entry)

@app.route('/delete_entry/<int:entry_id>', methods=['POST'])
def delete_entry(entry_id):
    entry = TimeEntry.query.get_or_404(entry_id)
    db.session.delete(entry)
    db.session.commit()
    flash('记录已删除', 'success')
    return redirect(url_for('time_entry'))

@app.route('/statistics')
def statistics():
    # 获取查询参数
    view_type = request.args.get('view_type', 'week')  # week, month, year
    date_str = request.args.get('date', datetime.now().strftime('%Y-%m-%d'))
    
    try:
        selected_date = datetime.strptime(date_str, '%Y-%m-%d')
    except ValueError:
        selected_date = datetime.now()
    
    if view_type == 'week':
        # 获取最近7天的记录
        end_date = selected_date
        start_date = end_date - timedelta(days=7)
        date_format = '%Y-%m-%d'
        group_format = '%Y-%m-%d'
    elif view_type == 'month':
        # 获取选定月份的记录
        start_date = selected_date.replace(day=1)
        if start_date.month == 12:
            end_date = start_date.replace(year=start_date.year + 1, month=1)
        else:
            end_date = start_date.replace(month=start_date.month + 1)
        date_format = '%Y-%m'
        group_format = '%Y-%m-%d'
    else:  # year
        # 获取选定年份的记录
        start_date = selected_date.replace(month=1, day=1)
        end_date = start_date.replace(year=start_date.year + 1)
        date_format = '%Y'
        group_format = '%Y-%m'

    entries = TimeEntry.query.filter(
        TimeEntry.start_time >= start_date,
        TimeEntry.start_time < end_date
    ).order_by(TimeEntry.start_time.desc()).all()

    # 按类别统计时间
    category_stats = {}
    for entry in entries:
        duration = (entry.end_time - entry.start_time).total_seconds() / 3600  # 转换为小时
        if entry.category in category_stats:
            category_stats[entry.category] += duration
        else:
            category_stats[entry.category] = duration

    # 按日期统计时间
    daily_stats = {}
    category_daily_stats = {}
    
    for entry in entries:
        date_key = entry.start_time.strftime(group_format)
        duration = (entry.end_time - entry.start_time).total_seconds() / 3600
        
        # 总时间统计
        if date_key in daily_stats:
            daily_stats[date_key] += duration
        else:
            daily_stats[date_key] = duration
        
        # 按类别的每日统计
        if date_key not in category_daily_stats:
            category_daily_stats[date_key] = {}
        
        if entry.category in category_daily_stats[date_key]:
            category_daily_stats[date_key][entry.category] += duration
        else:
            category_daily_stats[date_key][entry.category] = duration

    # 获取所有类别
    all_categories = list(category_stats.keys())
    
    # 准备图表数据
    dates = sorted(daily_stats.keys())
    daily_total_hours = [daily_stats[date] for date in dates]
    
    # 准备类别时间序列数据
    category_time_series = {}
    for category in all_categories:
        category_time_series[category] = []
        for date in dates:
            if date in category_daily_stats and category in category_daily_stats[date]:
                category_time_series[category].append(category_daily_stats[date][category])
            else:
                category_time_series[category].append(0)

    return render_template('statistics.html',
                         view_type=view_type,
                         selected_date=selected_date,
                         entries=entries,
                         category_stats=category_stats,
                         dates=dates,
                         daily_total_hours=daily_total_hours,
                         category_time_series=category_time_series,
                         all_categories=all_categories)

if __name__ == '__main__':
    with app.app_context():
        # 检查数据库文件是否存在
        db_file = os.path.join(basedir, "time_tracker.db")
        if not os.path.exists(db_file):
            db.create_all()  # 只在数据库不存在时创建表
    app.run(debug=True) 