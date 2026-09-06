variable "aws_region" {
	description = "region in which the tests are going to be executed"
	type = string
	default = "eu-west-1"
}

variable "aws_main_net" {
	description = "main network cidr block"
	type = string
	default = "10.0.0.0/16"
}

variable "allowed_cidr" {
	description = "ssh allowed cidr ipv4 addresses"
	type = string
}

variable "aws_subnet_cidr_block1" {
	description = "sub-network 1 cidr block"
	type = string
	default = "10.0.1.0/24"
}

variable "aws_main_node_name" {
	description = "main instance name"
	type = string
	default = "XACML-FHE-EVAL"
}

variable "aws_main_node_type" {
	description = "EC2 instance type"
	type = string
	default = "c8a.4xlarge"
}


variable "aws_instance_root_volume_size" {
	description = "Root disk volume size in Gib for EC2 intsances"
	type = number
	default = 20
}
///	change that to the appropriate key of your choice
variable "key_name" {
	description = "key name to use to establish ssh connection"
	type = string
}

variable "ssh_private_key_path" {
	description = "ssh private key path"
	type = string
	sensitive   = true
}