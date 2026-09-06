resource "aws_vpc" "xacml_fhe_eval" {
	cidr_block = var.aws_main_net
}

resource "aws_subnet" "main" {
	availability_zone = "eu-west-1b"
	vpc_id = aws_vpc.xacml_fhe_eval.id
	cidr_block = var.aws_subnet_cidr_block1
	map_public_ip_on_launch = true

	tags = {
		Name = "xacml-fhe-eval-vpc-main-subnet"
	}
}

resource "aws_internet_gateway" "main" {
	vpc_id = aws_vpc.xacml_fhe_eval.id
	tags = {
		Name = "xacml-fhe-eval-vpc-main-internet-gateway"
	}
}

resource "aws_route_table" "public" {
	vpc_id = aws_vpc.xacml_fhe_eval.id
	
	route {
		cidr_block = "0.0.0.0/0"
		gateway_id = aws_internet_gateway.main.id
	}

	tags = {
		Name = "xacml-fhe-eval-vpc-public-route-table"
	}
}

resource "aws_route_table_association" "public" {
	subnet_id = aws_subnet.main.id
	route_table_id = aws_route_table.public.id
}

resource "aws_security_group" "main" {
	provider = aws
	description =  "Allow ssh connection to this vpc and all outbound traffic from that vpc"
	vpc_id = aws_vpc.xacml_fhe_eval.id

	tags = {
		Name = "xacml-fhe-eval-vpc-security-group"
	}
}

resource "aws_vpc_security_group_ingress_rule" "allow_ssh" {
	security_group_id = aws_security_group.main.id
	cidr_ipv4 = var.allowed_cidr
	from_port = 22
	to_port = 22
	ip_protocol = "tcp"
	tags = {
		Name = "ingress-rule"
	}
}

resource "aws_vpc_security_group_egress_rule" "allow_all_outbound_traffic" {
	security_group_id = aws_security_group.main.id
	cidr_ipv4 = "0.0.0.0/0"
	ip_protocol = "-1"
	tags = {
		Name = "egress-rule"
	}
}

data "aws_ssm_parameter" "ubuntu_ami" {
	name = "/aws/service/canonical/ubuntu/server/24.04/stable/current/amd64/hvm/ebs-gp3/ami-id"
}

resource "aws_instance" "main_node" {
	ami = data.aws_ssm_parameter.ubuntu_ami.value
	instance_type = var.aws_main_node_type
	key_name = var.key_name
	vpc_security_group_ids = [ aws_security_group.main.id ]
	subnet_id = aws_subnet.main.id
	associate_public_ip_address = true


	root_block_device {
		volume_size = var.aws_instance_root_volume_size
		volume_type = "gp3"
		delete_on_termination = true
		encrypted = false
	}

	

	tags = {
		Name = "xacml-fhe-eval-main-node-instance"
	}

}

resource "terraform_data" "run_test_and_get_results" {

	connection {
		type = "ssh"
		user = "ubuntu"
		private_key = file(var.ssh_private_key_path)
		host = aws_instance.main_node.public_ip
	}

	provisioner "file" {
		source = "${path.module}/target/release/hpdp_funcs"
		destination = "/home/ubuntu/hpdp_funcs"
	}
	
	provisioner "file" {
		source = "${path.module}/main.py"
		destination = "/home/ubuntu/main.py"
	}

	provisioner "remote-exec" {
		inline = [ 
			"sudo chmod +x /home/ubuntu/hpdp_funcs",
			"sudo chmod +x /home/ubuntu/main.py",
			"sudo apt update",
			"sudo apt install -y python3 python3-pip python3-dev python3-matplotlib python3-pandas python3-numpy python3-scipy"
		]
	}

	provisioner "remote-exec" {
		inline = [ 
			"sudo nice -20 /home/ubuntu/hpdp_funcs bench"
		]
	}

	provisioner "remote-exec" {
		inline = [ 
			"python3 main.py"
		]
	}

	provisioner "local-exec" {
		command = <<-EOT
			scp -i "$SSH_KEY_PATH" \
        	-o StrictHostKeyChecking=accept-new \
			ubuntu@"$EC2_INSTANCE_IP":/home/ubuntu/metrics_ops.csv \
			metrics_ops.csv && \
			scp -i "$SSH_KEY_PATH" \
        	-o StrictHostKeyChecking=accept-new \
			ubuntu@"$EC2_INSTANCE_IP":/home/ubuntu/result.png \
			result.png
		EOT

		environment = {
			SSH_KEY_PATH = var.ssh_private_key_path
			EC2_INSTANCE_IP = aws_instance.main_node.public_ip
		}
	}

	depends_on = [ aws_instance.main_node ]
}